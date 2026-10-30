//! 导出设备内核镜像里 kallsyms 的全部符号偏移（相对 _text）。
//!
//! usage: cargo run --release --example dump_symbols -- <boot.img|Image> [out.txt]

use std::io::Write;
use std::path::Path;

use ghostlock_extract::boot::BootImage;
use ghostlock_extract::derive::relative_symbols;
use ghostlock_extract::kallsyms;
use ghostlock_extract::kallsyms_finder;

fn main() {
    let mut args = std::env::args().skip(1);
    let image = args.next().expect("image path");
    let out = args.next();

    let boot = BootImage::load(Path::new(&image)).expect("load image");
    let btf_at = boot.embedded_btf_at();
    let pair = btf_at.as_ref().map(|(o, b)| (*o, b.len()));
    let ks = kallsyms_finder::recover(&boot.kernel, pair).expect("kallsyms");
    let base = kallsyms::unique(&ks.symbols, "_text")
        .or_else(|| kallsyms::unique(&ks.symbols, "_head"))
        .expect("base");
    let (rel, _) = relative_symbols(&ks.symbols, base);

    let mut rows: Vec<(u64, String)> = Vec::new();
    for (name, offsets) in &rel {
        for off in offsets {
            rows.push((*off, name.clone()));
        }
    }
    rows.sort();
    rows.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    match out {
        Some(path) => {
            let mut f = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
            for (off, name) in &rows {
                writeln!(f, "{off} {name}").unwrap();
            }
            eprintln!("wrote {} symbols to {path}", rows.len());
        }
        None => {
            for (off, name) in &rows {
                println!("{off} {name}");
            }
        }
    }
}
