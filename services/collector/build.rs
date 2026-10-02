fn main() {
    // sqlx::migrate! embarque les fichiers SQL : un ajout doit reconstruire le migrateur.
    println!("cargo:rerun-if-changed=migrations");
}
