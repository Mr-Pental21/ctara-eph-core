use clap::{Parser, Subcommand};
use dhruv_vedic_base::{
    AyanamshaSystem, ayanamsha_deg, deg_to_dms, jd_tdb_to_centuries, nakshatra28_from_longitude,
    nakshatra28_from_tropical, nakshatra_from_longitude, nakshatra_from_tropical,
    rashi_from_longitude, rashi_from_tropical,
};

#[derive(Parser)]
#[command(name = "dhruv", about = "Dhruv ephemeris CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Rashi from sidereal longitude
    Rashi {
        /// Sidereal ecliptic longitude in degrees
        lon: f64,
    },
    /// Nakshatra from sidereal longitude
    Nakshatra {
        /// Sidereal ecliptic longitude in degrees
        lon: f64,
        /// Scheme: 27 (default) or 28
        #[arg(long, default_value = "27")]
        scheme: u32,
    },
    /// Rashi from tropical longitude + ayanamsha
    RashiTropical {
        /// Tropical ecliptic longitude in degrees
        lon: f64,
        /// Ayanamsha system code (0-19)
        #[arg(long)]
        ayanamsha: i32,
        /// Julian Date TDB
        #[arg(long)]
        jd: f64,
        /// Apply nutation correction
        #[arg(long)]
        nutation: bool,
    },
    /// Nakshatra from tropical longitude + ayanamsha
    NakshatraTropical {
        /// Tropical ecliptic longitude in degrees
        lon: f64,
        /// Ayanamsha system code (0-19)
        #[arg(long)]
        ayanamsha: i32,
        /// Julian Date TDB
        #[arg(long)]
        jd: f64,
        /// Apply nutation correction
        #[arg(long)]
        nutation: bool,
        /// Scheme: 27 (default) or 28
        #[arg(long, default_value = "27")]
        scheme: u32,
    },
    /// Convert degrees to DMS
    Dms {
        /// Angle in decimal degrees
        deg: f64,
    },
}

fn aya_system_from_code(code: i32) -> Option<AyanamshaSystem> {
    let all = AyanamshaSystem::all();
    let idx = usize::try_from(code).ok()?;
    all.get(idx).copied()
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Rashi { lon } => {
            let info = rashi_from_longitude(lon);
            let dms = info.dms;
            println!(
                "{} ({}) - {} deg {} min {:.1} sec ({:.4} deg in rashi)",
                info.rashi.name(),
                info.rashi.western_name(),
                dms.degrees,
                dms.minutes,
                dms.seconds,
                info.degrees_in_rashi
            );
        }

        Commands::Nakshatra { lon, scheme } => match scheme {
            27 => {
                let info = nakshatra_from_longitude(lon);
                println!(
                    "{} (index {}) - Pada {} ({:.4} deg in nakshatra, {:.4} deg in pada)",
                    info.nakshatra.name(),
                    info.nakshatra_index,
                    info.pada,
                    info.degrees_in_nakshatra,
                    info.degrees_in_pada
                );
            }
            28 => {
                let info = nakshatra28_from_longitude(lon);
                println!(
                    "{} (index {}) - Pada {} ({:.4} deg in nakshatra)",
                    info.nakshatra.name(),
                    info.nakshatra_index,
                    info.pada,
                    info.degrees_in_nakshatra
                );
            }
            _ => {
                eprintln!("Invalid scheme: {scheme}. Use 27 or 28.");
                std::process::exit(1);
            }
        },

        Commands::RashiTropical {
            lon,
            ayanamsha,
            jd,
            nutation,
        } => {
            let system = match aya_system_from_code(ayanamsha) {
                Some(s) => s,
                None => {
                    eprintln!("Invalid ayanamsha code: {ayanamsha} (0-19)");
                    std::process::exit(1);
                }
            };
            let t = jd_tdb_to_centuries(jd);
            let aya = ayanamsha_deg(system, t, nutation);
            let info = rashi_from_tropical(lon, system, jd, nutation);
            let dms = info.dms;
            println!(
                "Ayanamsha: {:.4} deg",
                aya
            );
            println!(
                "Sidereal: {:.4} deg",
                lon - aya
            );
            println!(
                "{} ({}) - {} deg {} min {:.1} sec ({:.4} deg in rashi)",
                info.rashi.name(),
                info.rashi.western_name(),
                dms.degrees,
                dms.minutes,
                dms.seconds,
                info.degrees_in_rashi
            );
        }

        Commands::NakshatraTropical {
            lon,
            ayanamsha,
            jd,
            nutation,
            scheme,
        } => {
            let system = match aya_system_from_code(ayanamsha) {
                Some(s) => s,
                None => {
                    eprintln!("Invalid ayanamsha code: {ayanamsha} (0-19)");
                    std::process::exit(1);
                }
            };
            let t = jd_tdb_to_centuries(jd);
            let aya = ayanamsha_deg(system, t, nutation);
            println!(
                "Ayanamsha: {:.4} deg",
                aya
            );
            println!(
                "Sidereal: {:.4} deg",
                lon - aya
            );
            match scheme {
                27 => {
                    let info = nakshatra_from_tropical(lon, system, jd, nutation);
                    println!(
                        "{} (index {}) - Pada {} ({:.4} deg in nakshatra, {:.4} deg in pada)",
                        info.nakshatra.name(),
                        info.nakshatra_index,
                        info.pada,
                        info.degrees_in_nakshatra,
                        info.degrees_in_pada
                    );
                }
                28 => {
                    let info = nakshatra28_from_tropical(lon, system, jd, nutation);
                    println!(
                        "{} (index {}) - Pada {} ({:.4} deg in nakshatra)",
                        info.nakshatra.name(),
                        info.nakshatra_index,
                        info.pada,
                        info.degrees_in_nakshatra
                    );
                }
                _ => {
                    eprintln!("Invalid scheme: {scheme}. Use 27 or 28.");
                    std::process::exit(1);
                }
            }
        }

        Commands::Dms { deg } => {
            let d = deg_to_dms(deg);
            println!("{} deg {} min {:.2} sec", d.degrees, d.minutes, d.seconds);
        }
    }
}
