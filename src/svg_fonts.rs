use std::sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use usvg::fontdb;

// I'm not happy with this solution but I guess it is better then nothing...
// Would be better if is was passed into widgets by masonry but I don't know
// if is it possible.
static THEME: LazyLock<RwLock<fontdb::Database>> = LazyLock::new(|| {
    let mut fontdb = fontdb::Database::default();
    fontdb.load_system_fonts();

    // TODO: Add default fonts into the package so they are always present.
    fontdb.set_serif_family("Times New Roman");
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_cursive_family("Comic Sans MS");
    fontdb.set_fantasy_family("Impact");
    fontdb.set_monospace_family("Courier New");

    // FIXME: FIXME FIXME: I'm not sure about the legality of the fonts
    // being committed in the repo. Needs to be resolved ASAP.

    // TDDO: This should point to some asset dir.
    fontdb.load_fonts_dir("./fonts/");
    RwLock::new(fontdb)
});

pub fn get_svg_fonts<'a>() -> RwLockReadGuard<'a, fontdb::Database> {
    (*THEME).read().unwrap()
}

pub fn get_svg_fonts_but<'a>() -> RwLockWriteGuard<'a, fontdb::Database> {
    (*THEME).write().unwrap()
}
