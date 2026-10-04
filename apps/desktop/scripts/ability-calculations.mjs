// Sous-ensemble symbolique : un nœud inconnu invalide toute la formule, jamais un total partiel.
const object = value => value && typeof value === 'object' && !Array.isArray(value);
const finite = value => typeof value === 'number' && Number.isFinite(value) && Math.abs(value) < 1e9;
const round = value => Math.round(value * 100000) / 100000;
const allowed = (node, keys) => Object.keys(node).every(key => key === '__type' || keys.includes(key));
const compact = values => values.every(value => value === values[0]) ? [values[0]] : values;

export function calculationHash(name) {
  let hash = 2166136261;
  for (const char of name.toLowerCase()) hash = Math.imul(hash ^ char.charCodeAt(0), 16777619) >>> 0;
  return `{${hash.toString(16).padStart(8, '0')}}`;
}
const matches = (key, name) => key.toLowerCase() === name.toLowerCase() || key.toLowerCase() === calculationHash(name);

export function resolveAbilityVariable(spell, expression, ranks) {
  if (!object(spell) || !Number.isInteger(ranks) || ranks < 1 || ranks > 6) return null;
  const match = /^([a-z0-9_{}]+)(?:\*([-+]?\d+(?:\.\d+)?))?$/i.exec(expression.trim());
  if (!match) return null;
  const readValues = values => {
    if (!Array.isArray(values) || values.length < ranks + 1) return null;
    const selected = values.slice(1, ranks + 1);
    return selected.every(finite) ? compact(selected.map(round)) : null;
  };
  const data = name => {
    const entries = (spell.DataValues ?? spell.mDataValues ?? []).filter?.(entry => typeof entry.name === 'string' && matches(entry.name, name));
    return entries?.length === 1 ? readValues(entries[0].values) : null;
  };
  const scalar = values => values ? [{values}] : null;
  const scale = (terms, multiplier) => terms && finite(multiplier) ? terms.map(term => ({...term, values: compact(term.values.map(value => round(value * multiplier)))})) : null;
  const statTerm = (node, values) => {
    // Défaut total et 2=bonus corroborés par QTotalADRatio (Aatrox) / BonusADRatio (Talon), patch 16.19.
    const stat = node.mStat === undefined ? 0 : node.mStat, formula = node.mStatFormula === undefined ? 0 : node.mStatFormula;
    const key = stat === 0 && formula === 0 ? 'ability_power' : stat === 2 && formula === 0 ? 'attack_damage' : stat === 2 && formula === 2 ? 'bonus_attack_damage' : null;
    return key && values ? [{values, stat:key}] : null;
  };
  const concat = parts => parts.every(Boolean) ? parts.flat() : null;
  const part = (node, depth = 0) => {
    if (!object(node) || depth > 12) return null;
    switch (node.__type) {
      case 'NumberCalculationPart': return allowed(node,['mNumber']) && finite(node.mNumber) ? scalar([round(node.mNumber)]) : null;
      case 'NamedDataValueCalculationPart': return allowed(node,['mDataValue']) && typeof node.mDataValue === 'string' ? scalar(data(node.mDataValue)) : null;
      case 'EffectValueCalculationPart': return allowed(node,['mEffectIndex']) && Number.isInteger(node.mEffectIndex) && node.mEffectIndex > 0 ? scalar(readValues(spell.mEffectAmount?.[node.mEffectIndex - 1]?.value)) : null;
      case 'StatByCoefficientCalculationPart': return allowed(node,['mCoefficient','mStat','mStatFormula']) && finite(node.mCoefficient) ? statTerm(node,[round(node.mCoefficient)]) : null;
      case 'StatByNamedDataValueCalculationPart': return allowed(node,['mDataValue','mStat','mStatFormula']) && typeof node.mDataValue === 'string' ? statTerm(node,data(node.mDataValue)) : null;
      case 'SumOfSubPartsCalculationPart': return allowed(node,['mSubparts']) && Array.isArray(node.mSubparts) && node.mSubparts.length > 0 ? concat(node.mSubparts.map(child => part(child,depth + 1))) : null;
      default: return null;
    }
  };
  const calculation = (name, seen = new Set()) => {
    if (seen.has(name.toLowerCase()) || seen.size > 12) return null;
    const keys = Object.keys(spell.mSpellCalculations ?? {}).filter(key => matches(key,name));
    if (!keys.length) return scalar(data(name));
    if (keys.length !== 1) return null;
    const node = spell.mSpellCalculations[keys[0]];
    if (!object(node)) return null;
    let terms;
    if (node.__type === 'GameCalculation' && allowed(node,['mFormulaParts','mMultiplier','mSimpleTooltipCalculationDisplay','mPrecision']) && Array.isArray(node.mFormulaParts) && node.mFormulaParts.length > 0) {
      terms = concat(node.mFormulaParts.map(child => part(child)));
    } else if (node.__type === 'GameCalculationModified' && allowed(node,['mModifiedGameCalculation','mMultiplier','mSimpleTooltipCalculationDisplay','mPrecision']) && typeof node.mModifiedGameCalculation === 'string') {
      terms = calculation(node.mModifiedGameCalculation,new Set([...seen,name.toLowerCase()]));
    } else return null;
    if (Object.hasOwn(node,'mMultiplier')) {
      const multiplier = part(node.mMultiplier);
      if (multiplier?.length !== 1 || multiplier[0].stat || multiplier[0].values.length !== 1) return null;
      terms = scale(terms,multiplier[0].values[0]);
    }
    return terms;
  };
  const terms = scale(calculation(match[1]),match[2] === undefined ? 1 : Number(match[2]));
  return terms?.length && terms.length <= 32 && terms.every(term => term.values.every(finite)) ? {terms} : null;
}
