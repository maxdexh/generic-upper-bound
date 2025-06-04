use std::{
    env,
    error::Error,
    fmt,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=build.rs");

    let ptr_width: u16 = env::var("CARGO_CFG_TARGET_POINTER_WIDTH")?
        .parse::<u64>()?
        .try_into()?;

    let out_dir = env::var_os("OUT_DIR").ok_or(env::VarError::NotPresent)?;
    let out_dir = Path::new(&out_dir);

    let mut f = BufWriter::new(File::create(out_dir.join("for_each_size.rs"))?);
    write_for_each_size_macro(&mut f, ptr_width)?;

    Ok(())
}

fn write_for_each_size_macro(f: &mut impl Write, ptr_width: u16) -> std::io::Result<()> {
    write!(f, "macro_rules! for_each_size {{")?;
    write!(f, "{}($($mac:tt)*) => {{", IndentLn(1))?;
    write!(f, "{}$($mac)*! {{", IndentLn(2))?;

    let ln = IndentLn(3);
    // yield 0 and 1
    write!(f, "{ln}0")?;
    write!(f, "{ln}1")?;
    // yield all n = p * pow(2, i - 1), p = 2 or 3, i in 1..ptr_width
    for i in 1..ptr_width {
        // in binary, n looks like 0b1000...00 or 0b1100...00
        for part in 0..=1u8 {
            write!(f, "{ln}0b1")?;
            write!(f, "{part}")?;
            for _ in 1..i {
                write!(f, "0")?;
            }
        }
    }
    let usize_max = "0b"
        .chars()
        .chain("1".repeat(ptr_width.into()).chars())
        .collect::<String>();
    write!(f, "{ln}{usize_max}")?;

    for i in (0..=2).rev() {
        write!(f, "{}", IndentLn(i))?;
        write!(f, "}}")?;
    }
    Ok(())
}
struct IndentLn(u8);
impl std::fmt::Display for IndentLn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        for _ in 0..self.0 {
            write!(f, "    ")?;
        }
        Ok(())
    }
}
