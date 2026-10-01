//! CDN synthétique réservé aux tests ; aucune requête vers Riot.
use super::{StaticError, StaticResponse, StaticTransport};
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    scripts: BTreeMap<String, VecDeque<StaticResponse>>,
    calls: Vec<String>,
    versions: Vec<String>,
    live: String,
    denied: Option<String>,
}

#[derive(Clone, Default)]
pub struct FakeCdn {
    state: Arc<Mutex<State>>,
}

impl FakeCdn {
    pub fn new() -> Self {
        let fake = Self::default();
        fake.versions(&["16.20.1", "16.19.1", "16.18.1"], "16.19.1");
        fake
    }
    pub fn versions(&self, versions: &[&str], live: &str) {
        let mut s = self.state.lock().unwrap();
        s.versions = versions.iter().map(|s| (*s).to_owned()).collect();
        s.live = live.to_owned();
    }
    pub fn deny(&self, fragment: &str) {
        self.state.lock().unwrap().denied = Some(fragment.into());
    }
    pub fn script(&self, url: &str, responses: Vec<StaticResponse>) {
        self.state
            .lock()
            .unwrap()
            .scripts
            .insert(url.into(), responses.into());
    }
    pub fn calls(&self) -> Vec<String> {
        self.state.lock().unwrap().calls.clone()
    }
}

pub fn response(status: u16, data: Value) -> StaticResponse {
    StaticResponse {
        status,
        body: serde_json::to_vec(&data).unwrap(),
    }
}

fn champion(id: &str, key: &str, detail: bool) -> Value {
    let mut result = json!({"id":id,"key":key,"name":id,"image":{"full":format!("{id}.png"),"group":"champion"}});
    if detail {
        result["spells"] = json!((0..4).map(|n|json!({"id":format!("{id}{n}"),"name":"Sort","description":"Description synthétique","image":{"full":format!("spell{n}.png"),"group":"spell"}})).collect::<Vec<_>>());
        result["passive"] = json!({"name":"Passif","description":"Passif synthétique","image":{"full":"passive.png","group":"passive"}});
    }
    result
}

fn document(version: &str, resource: &str) -> Option<Value> {
    let (kind, data) = match resource {
        "champion.json" => ("champion", json!({"Aatrox":champion("Aatrox","266",false)})),
        "champion/Aatrox.json" => ("champion", json!({"Aatrox":champion("Aatrox","266",true)})),
        "mode/classic/champion.json" => (
            "champion",
            json!({"Jade_Ahri":champion("Jade_Ahri","60103",false)}),
        ),
        "mode/classic/champion/Jade_Ahri.json" => (
            "champion",
            json!({"Jade_Ahri":champion("Jade_Ahri","60103",true)}),
        ),
        "item.json" => (
            "item",
            json!({"1001":{"name":"Bottes","description":"Description","image":{"full":"1001.png","group":"item"}}}),
        ),
        "summoner.json" => (
            "summoner",
            json!({"SummonerFlash":{"id":"SummonerFlash","key":"4","name":"Saut éclair","image":{"full":"SummonerFlash.png","group":"spell"}}}),
        ),
        "map.json" => (
            "map",
            json!({"11":{"MapId":"11","MapName":"Faille","image":{"full":"map11.png","group":"map"}}}),
        ),
        "profileicon.json" => (
            "profileicon",
            json!({"0":{"id":0,"image":{"full":"0.png","group":"profileicon"}}}),
        ),
        "runesReforged.json" => {
            return Some(
                json!([{"id":8000,"key":"Precision","name":"Précision","icon":"perk-images/Styles/7201_Precision.png","slots":[{"runes":[{"id":8005,"key":"PressTheAttack","name":"Attaque soutenue","icon":"perk-images/Styles/Precision/PressTheAttack/PressTheAttack.png","shortDesc":"Description","longDesc":"Description longue"}]}]}]),
            )
        }
        _ => return None,
    };
    Some(json!({"type":kind,"version":version,"data":data}))
}

impl StaticTransport for FakeCdn {
    async fn get(&self, url: &str) -> Result<StaticResponse, StaticError> {
        let mut s = self.state.lock().unwrap();
        s.calls.push(url.into());
        if let Some(reply) = s.scripts.get_mut(url).and_then(VecDeque::pop_front) {
            return Ok(reply);
        }
        if s.denied.as_ref().is_some_and(|part| url.contains(part)) {
            return Ok(response(503, json!({"private":"never surface this"})));
        }
        let data = match url {
            "https://ddragon.leagueoflegends.com/api/versions.json" => json!(s.versions),
            "https://ddragon.leagueoflegends.com/realms/euw.json" => json!({"v":s.live}),
            "https://static.developer.riotgames.com/docs/lol/queues.json" => {
                json!([{"queueId":420,"map":"Summoner's Rift","description":"Ranked Solo","notes":null}])
            }
            "https://static.developer.riotgames.com/docs/lol/maps.json" => {
                json!([{"mapId":11,"mapName":"Summoner's Rift","notes":"Current"}])
            }
            "https://static.developer.riotgames.com/docs/lol/gameModes.json" => {
                json!([{"gameMode":"CLASSIC","description":"Classic"}])
            }
            "https://static.developer.riotgames.com/docs/lol/gameTypes.json" => {
                json!([{"gametype":"MATCHED_GAME","description":"Matchmade"}])
            }
            _ => {
                let path = url
                    .strip_prefix("https://ddragon.leagueoflegends.com/cdn/")
                    .ok_or(StaticError::Network)?;
                let parts: Vec<_> = path.splitn(4, '/').collect();
                if parts.len() != 4 || parts[1] != "data" || !["fr_FR", "en_US"].contains(&parts[2])
                {
                    return Err(StaticError::Network);
                }
                document(parts[0], parts[3]).ok_or(StaticError::Network)?
            }
        };
        Ok(response(200, data))
    }
}
