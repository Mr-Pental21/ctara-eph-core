use std::path::PathBuf;

use clap::{Parser, Subcommand};
use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::{
    AyanamshaSystem, ayanamsha_deg, deg_to_dms, jd_tdb_to_centuries, nakshatra28_from_longitude,
    nakshatra28_from_tropical, nakshatra_from_longitude, nakshatra_from_tropical,
    rashi_from_longitude, rashi_from_tropical,
};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};

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
    /// Find next Purnima (full moon)
    NextPurnima {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Path to SPK kernel (de442s.bsp)
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel (naif0012.tls)
        #[arg(long)]
        lsk: PathBuf,
    },
    /// Find next Amavasya (new moon)
    NextAmavasya {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
    },
    /// Find next Sankranti (Sun entering a rashi)
    NextSankranti {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Ayanamsha system code (0-19, default 0=Lahiri)
        #[arg(long, default_value = "0")]
        ayanamsha: i32,
        /// Apply nutation correction
        #[arg(long)]
        nutation: bool,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
    },
    /// Determine the Masa (lunar month) for a date
    Masa {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Ayanamsha system code (0-19, default 0=Lahiri)
        #[arg(long, default_value = "0")]
        ayanamsha: i32,
        /// Apply nutation correction
        #[arg(long)]
        nutation: bool,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
    },
    /// Determine the Ayana (Uttarayana/Dakshinayana) for a date
    Ayana {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Ayanamsha system code (0-19, default 0=Lahiri)
        #[arg(long, default_value = "0")]
        ayanamsha: i32,
        /// Apply nutation correction
        #[arg(long)]
        nutation: bool,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
    },
    /// Determine the Varsha (60-year samvatsara cycle) for a date
    Varsha {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Ayanamsha system code (0-19, default 0=Lahiri)
        #[arg(long, default_value = "0")]
        ayanamsha: i32,
        /// Apply nutation correction
        #[arg(long)]
        nutation: bool,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
    },
    /// Determine the Tithi (lunar day) for a date
    Tithi {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
    },
    /// Determine the Karana (half-tithi) for a date
    Karana {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
    },
    /// Determine the Yoga (luni-solar yoga) for a date
    Yoga {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Ayanamsha system code (0-19, default 0=Lahiri)
        #[arg(long, default_value = "0")]
        ayanamsha: i32,
        /// Apply nutation correction
        #[arg(long)]
        nutation: bool,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
    },
    /// Determine the Vaar (Vedic weekday) for a date and location
    Vaar {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Latitude in degrees (north positive)
        #[arg(long)]
        lat: f64,
        /// Longitude in degrees (east positive)
        #[arg(long)]
        lon: f64,
        /// Altitude in meters (default 0)
        #[arg(long, default_value = "0")]
        alt: f64,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
        /// Path to IERS EOP file (finals2000A.all)
        #[arg(long)]
        eop: PathBuf,
    },
    /// Determine the Hora (planetary hour) for a date and location
    Hora {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Latitude in degrees (north positive)
        #[arg(long)]
        lat: f64,
        /// Longitude in degrees (east positive)
        #[arg(long)]
        lon: f64,
        /// Altitude in meters (default 0)
        #[arg(long, default_value = "0")]
        alt: f64,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
        /// Path to IERS EOP file (finals2000A.all)
        #[arg(long)]
        eop: PathBuf,
    },
    /// Determine the Ghatika (1-60) for a date and location
    Ghatika {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Latitude in degrees (north positive)
        #[arg(long)]
        lat: f64,
        /// Longitude in degrees (east positive)
        #[arg(long)]
        lon: f64,
        /// Altitude in meters (default 0)
        #[arg(long, default_value = "0")]
        alt: f64,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
        /// Path to IERS EOP file (finals2000A.all)
        #[arg(long)]
        eop: PathBuf,
    },
    /// Compute all 16 sphutas for a date and location
    Sphutas {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Latitude in degrees (north positive)
        #[arg(long)]
        lat: f64,
        /// Longitude in degrees (east positive)
        #[arg(long)]
        lon: f64,
        /// Altitude in meters (default 0)
        #[arg(long, default_value = "0")]
        alt: f64,
        /// Ayanamsha system code (0-19, default 0=Lahiri)
        #[arg(long, default_value = "0")]
        ayanamsha: i32,
        /// Apply nutation correction
        #[arg(long)]
        nutation: bool,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
        /// Path to IERS EOP file (finals2000A.all)
        #[arg(long)]
        eop: PathBuf,
    },
    /// Combined panchang: tithi, karana, yoga, vaar, hora, ghatika
    Panchang {
        /// UTC datetime (YYYY-MM-DDThh:mm:ssZ)
        #[arg(long)]
        date: String,
        /// Latitude in degrees (north positive)
        #[arg(long)]
        lat: f64,
        /// Longitude in degrees (east positive)
        #[arg(long)]
        lon: f64,
        /// Altitude in meters (default 0)
        #[arg(long, default_value = "0")]
        alt: f64,
        /// Ayanamsha system code (0-19, default 0=Lahiri)
        #[arg(long, default_value = "0")]
        ayanamsha: i32,
        /// Apply nutation correction
        #[arg(long)]
        nutation: bool,
        /// Path to SPK kernel
        #[arg(long)]
        bsp: PathBuf,
        /// Path to leap second kernel
        #[arg(long)]
        lsk: PathBuf,
        /// Path to IERS EOP file (finals2000A.all)
        #[arg(long)]
        eop: PathBuf,
    },
}

fn aya_system_from_code(code: i32) -> Option<AyanamshaSystem> {
    let all = AyanamshaSystem::all();
    let idx = usize::try_from(code).ok()?;
    all.get(idx).copied()
}

fn parse_utc(s: &str) -> Result<UtcTime, String> {
    // Parse "YYYY-MM-DDThh:mm:ssZ" or "YYYY-MM-DDThh:mm:ss"
    let s = s.trim_end_matches('Z');
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return Err(format!("expected YYYY-MM-DDThh:mm:ssZ, got {s}"));
    }
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return Err(format!("invalid date/time format: {s}"));
    }
    let year: i32 = date_parts[0].parse().map_err(|e| format!("{e}"))?;
    let month: u32 = date_parts[1].parse().map_err(|e| format!("{e}"))?;
    let day: u32 = date_parts[2].parse().map_err(|e| format!("{e}"))?;
    let hour: u32 = time_parts[0].parse().map_err(|e| format!("{e}"))?;
    let minute: u32 = time_parts[1].parse().map_err(|e| format!("{e}"))?;
    let second: f64 = time_parts[2].parse().map_err(|e| format!("{e}"))?;
    Ok(UtcTime::new(year, month, day, hour, minute, second))
}

fn load_engine(bsp: &PathBuf, lsk: &PathBuf) -> Engine {
    let config = EngineConfig::with_single_spk(bsp.clone(), lsk.clone(), 256, true);
    Engine::new(config).unwrap_or_else(|e| {
        eprintln!("Failed to load engine: {e}");
        std::process::exit(1);
    })
}

fn require_aya_system(code: i32) -> AyanamshaSystem {
    aya_system_from_code(code).unwrap_or_else(|| {
        eprintln!("Invalid ayanamsha code: {code} (0-19)");
        std::process::exit(1);
    })
}

fn load_eop(path: &PathBuf) -> EopKernel {
    EopKernel::load(path).unwrap_or_else(|e| {
        eprintln!("Failed to load EOP: {e}");
        std::process::exit(1);
    })
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
            let system = require_aya_system(ayanamsha);
            let t = jd_tdb_to_centuries(jd);
            let aya = ayanamsha_deg(system, t, nutation);
            let info = rashi_from_tropical(lon, system, jd, nutation);
            let dms = info.dms;
            println!("Ayanamsha: {:.4} deg", aya);
            println!("Sidereal: {:.4} deg", lon - aya);
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
            let system = require_aya_system(ayanamsha);
            let t = jd_tdb_to_centuries(jd);
            let aya = ayanamsha_deg(system, t, nutation);
            println!("Ayanamsha: {:.4} deg", aya);
            println!("Sidereal: {:.4} deg", lon - aya);
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

        Commands::NextPurnima { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            match dhruv_search::next_purnima(&engine, &utc) {
                Ok(Some(ev)) => {
                    println!("Next Purnima: {}", ev.utc);
                    println!("  Moon lon: {:.4} deg  Sun lon: {:.4} deg", ev.moon_longitude_deg, ev.sun_longitude_deg);
                }
                Ok(None) => println!("No Purnima found in search range"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextAmavasya { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            match dhruv_search::next_amavasya(&engine, &utc) {
                Ok(Some(ev)) => {
                    println!("Next Amavasya: {}", ev.utc);
                    println!("  Moon lon: {:.4} deg  Sun lon: {:.4} deg", ev.moon_longitude_deg, ev.sun_longitude_deg);
                }
                Ok(None) => println!("No Amavasya found in search range"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::NextSankranti {
            date,
            ayanamsha,
            nutation,
            bsp,
            lsk,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(ayanamsha);
            let engine = load_engine(&bsp, &lsk);
            let config = SankrantiConfig::new(system, nutation);
            match dhruv_search::next_sankranti(&engine, &utc, &config) {
                Ok(Some(ev)) => {
                    println!("Next Sankranti: {} ({})", ev.rashi.name(), ev.rashi.western_name());
                    println!("  Time: {}", ev.utc);
                    println!("  Sidereal lon: {:.4} deg  Tropical lon: {:.4} deg", ev.sun_sidereal_longitude_deg, ev.sun_tropical_longitude_deg);
                }
                Ok(None) => println!("No Sankranti found in search range"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Masa {
            date,
            ayanamsha,
            nutation,
            bsp,
            lsk,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(ayanamsha);
            let engine = load_engine(&bsp, &lsk);
            let config = SankrantiConfig::new(system, nutation);
            match dhruv_search::masa_for_date(&engine, &utc, &config) {
                Ok(info) => {
                    let adhika_str = if info.adhika { " (Adhika)" } else { "" };
                    println!("Masa: {}{}", info.masa.name(), adhika_str);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Ayana {
            date,
            ayanamsha,
            nutation,
            bsp,
            lsk,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(ayanamsha);
            let engine = load_engine(&bsp, &lsk);
            let config = SankrantiConfig::new(system, nutation);
            match dhruv_search::ayana_for_date(&engine, &utc, &config) {
                Ok(info) => {
                    println!("Ayana: {}", info.ayana.name());
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Varsha {
            date,
            ayanamsha,
            nutation,
            bsp,
            lsk,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(ayanamsha);
            let engine = load_engine(&bsp, &lsk);
            let config = SankrantiConfig::new(system, nutation);
            match dhruv_search::varsha_for_date(&engine, &utc, &config) {
                Ok(info) => {
                    println!("Samvatsara: {} (#{} in 60-year cycle)", info.samvatsara.name(), info.order);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Tithi { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            match dhruv_search::tithi_for_date(&engine, &utc) {
                Ok(info) => {
                    println!("Tithi: {} (index {})", info.tithi.name(), info.tithi_index);
                    println!("  Paksha: {}  Tithi in paksha: {}", info.paksha.name(), info.tithi_in_paksha);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Karana { date, bsp, lsk } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            match dhruv_search::karana_for_date(&engine, &utc) {
                Ok(info) => {
                    println!("Karana: {} (sequence index {})", info.karana.name(), info.karana_index);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Yoga {
            date,
            ayanamsha,
            nutation,
            bsp,
            lsk,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(ayanamsha);
            let engine = load_engine(&bsp, &lsk);
            let config = SankrantiConfig::new(system, nutation);
            match dhruv_search::yoga_for_date(&engine, &utc, &config) {
                Ok(info) => {
                    println!("Yoga: {} (index {})", info.yoga.name(), info.yoga_index);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Vaar {
            date,
            lat,
            lon,
            alt,
            bsp,
            lsk,
            eop,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let eop_kernel = load_eop(&eop);
            let location = GeoLocation::new(lat, lon, alt);
            let rs_config = RiseSetConfig::default();
            match dhruv_search::vaar_for_date(&engine, &eop_kernel, &utc, &location, &rs_config) {
                Ok(info) => {
                    println!("Vaar: {}", info.vaar.name());
                    println!("  Start (sunrise): {}", info.start);
                    println!("  End (next sunrise): {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Hora {
            date,
            lat,
            lon,
            alt,
            bsp,
            lsk,
            eop,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let eop_kernel = load_eop(&eop);
            let location = GeoLocation::new(lat, lon, alt);
            let rs_config = RiseSetConfig::default();
            match dhruv_search::hora_for_date(&engine, &eop_kernel, &utc, &location, &rs_config) {
                Ok(info) => {
                    println!("Hora: {} (position {} of 24)", info.hora.name(), info.hora_index);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Ghatika {
            date,
            lat,
            lon,
            alt,
            bsp,
            lsk,
            eop,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let engine = load_engine(&bsp, &lsk);
            let eop_kernel = load_eop(&eop);
            let location = GeoLocation::new(lat, lon, alt);
            let rs_config = RiseSetConfig::default();
            match dhruv_search::ghatika_for_date(&engine, &eop_kernel, &utc, &location, &rs_config) {
                Ok(info) => {
                    println!("Ghatika: {}/60", info.value);
                    println!("  Start: {}", info.start);
                    println!("  End:   {}", info.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Sphutas {
            date,
            lat,
            lon,
            alt,
            ayanamsha,
            nutation,
            bsp,
            lsk,
            eop,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(ayanamsha);
            let engine = load_engine(&bsp, &lsk);
            let eop_kernel = load_eop(&eop);
            let location = GeoLocation::new(lat, lon, alt);

            // Get graha sidereal longitudes
            let jd_tdb = utc.to_jd_tdb(engine.lsk());
            let graha_lons = dhruv_search::graha_sidereal_longitudes(&engine, jd_tdb, system, nutation)
                .unwrap_or_else(|e| {
                    eprintln!("Error computing graha longitudes: {e}");
                    std::process::exit(1);
                });

            // Get lagna (sidereal)
            let jd_utc = jd_tdb; // approximate; for more precision would use LSK
            let asc_rad = dhruv_vedic_base::ascendant_longitude_rad(engine.lsk(), &eop_kernel, &location, jd_utc)
                .unwrap_or_else(|e| {
                    eprintln!("Error computing lagna: {e}");
                    std::process::exit(1);
                });
            let t = dhruv_vedic_base::jd_tdb_to_centuries(jd_tdb);
            let aya = dhruv_vedic_base::ayanamsha_deg(system, t, nutation);
            let lagna_sid = (asc_rad.to_degrees() - aya).rem_euclid(360.0);

            // Get 8th lord longitude
            let lagna_rashi_idx = (lagna_sid / 30.0).floor() as u8;
            let eighth_rashi_idx = dhruv_vedic_base::nth_rashi_from(lagna_rashi_idx, 8);
            let eighth_lord = dhruv_vedic_base::rashi_lord_by_index(eighth_rashi_idx).unwrap();
            let eighth_lord_lon = graha_lons.longitude(eighth_lord);

            // Build sphuta inputs (gulika = 0 for now, as it requires upagraha computation)
            let inputs = dhruv_vedic_base::SphutalInputs {
                sun: graha_lons.longitude(dhruv_vedic_base::Graha::Surya),
                moon: graha_lons.longitude(dhruv_vedic_base::Graha::Chandra),
                mars: graha_lons.longitude(dhruv_vedic_base::Graha::Mangal),
                jupiter: graha_lons.longitude(dhruv_vedic_base::Graha::Guru),
                venus: graha_lons.longitude(dhruv_vedic_base::Graha::Shukra),
                rahu: graha_lons.longitude(dhruv_vedic_base::Graha::Rahu),
                lagna: lagna_sid,
                eighth_lord: eighth_lord_lon,
                gulika: 0.0,
            };

            let results = dhruv_vedic_base::all_sphutas(&inputs);
            println!("Sphutas for {} at {:.4}°N, {:.4}°E\n", date, lat, lon);
            println!("Graha longitudes (sidereal, aya code={} {}):",
                     ayanamsha, if nutation { "+nutation" } else { "" });
            for graha in dhruv_vedic_base::graha::ALL_GRAHAS {
                println!("  {:8} {:>8.4}°", graha.name(), graha_lons.longitude(graha));
            }
            println!("  {:8} {:>8.4}°\n", "Lagna", lagna_sid);
            println!("Sphutas:");
            for (sphuta, lon) in &results {
                let rashi_info = dhruv_vedic_base::rashi_from_longitude(*lon);
                println!("  {:24} {:>8.4}° ({} {}°{:02}'{:04.1}\")",
                    sphuta.name(), lon,
                    rashi_info.rashi.name(),
                    rashi_info.dms.degrees,
                    rashi_info.dms.minutes,
                    rashi_info.dms.seconds,
                );
            }
            println!("\nNote: Gulika=0° (placeholder until upagraha computation is available).");
            println!("  TriSphuta, ChatusSphuta, PanchaSphuta, SookshmaTrisphuta depend on Gulika.");
        }

        Commands::Panchang {
            date,
            lat,
            lon,
            alt,
            ayanamsha,
            nutation,
            bsp,
            lsk,
            eop,
        } => {
            let utc = parse_utc(&date).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let system = require_aya_system(ayanamsha);
            let engine = load_engine(&bsp, &lsk);
            let eop_kernel = load_eop(&eop);
            let location = GeoLocation::new(lat, lon, alt);
            let rs_config = RiseSetConfig::default();
            let config = SankrantiConfig::new(system, nutation);
            match dhruv_search::panchang_for_date(&engine, &eop_kernel, &utc, &location, &rs_config, &config) {
                Ok(info) => {
                    println!("Panchang for {} at {:.4}°N, {:.4}°E\n", date, lat, lon);
                    println!("Tithi:    {} (index {})", info.tithi.tithi.name(), info.tithi.tithi_index);
                    println!("  Paksha: {}  Tithi in paksha: {}", info.tithi.paksha.name(), info.tithi.tithi_in_paksha);
                    println!("  Start:  {}  End: {}", info.tithi.start, info.tithi.end);
                    println!("Karana:   {} (sequence {})", info.karana.karana.name(), info.karana.karana_index);
                    println!("  Start:  {}  End: {}", info.karana.start, info.karana.end);
                    println!("Yoga:     {} (index {})", info.yoga.yoga.name(), info.yoga.yoga_index);
                    println!("  Start:  {}  End: {}", info.yoga.start, info.yoga.end);
                    println!("Vaar:     {}", info.vaar.vaar.name());
                    println!("  Start:  {}  End: {}", info.vaar.start, info.vaar.end);
                    println!("Hora:     {} (position {} of 24)", info.hora.hora.name(), info.hora.hora_index);
                    println!("  Start:  {}  End: {}", info.hora.start, info.hora.end);
                    println!("Ghatika:  {}/60", info.ghatika.value);
                    println!("  Start:  {}  End: {}", info.ghatika.start, info.ghatika.end);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
