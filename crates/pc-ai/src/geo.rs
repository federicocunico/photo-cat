//! Geolocalizzazione offline: città più vicina alle coordinate GPS (GeoNames)
//! ed euristica sul nome delle cartelle quando il GPS manca.

use pc_core::{AppError, AppResult, ErrorCode};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct City {
    pub name: String,
    /// Nome originale GeoNames (spesso inglese), usato anche per la ricerca.
    pub alt_name: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
    pub population: u64,
    /// Fuso IANA della città (es. `Europe/Rome`), se presente nel file.
    pub timezone: Option<String>,
}

/// K-d tree su coordinate cartesiane della sfera: gestisce anche poli e antimeridiano.
pub struct Gazetteer {
    cities: Vec<City>,
    points: Vec<[f64; 3]>,
    root: Option<Box<Node>>,
    by_name: HashMap<String, usize>,
}

struct Node {
    index: usize,
    axis: usize,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}
fn point(lat: f64, lon: f64) -> [f64; 3] {
    let (p, l) = (lat.to_radians(), lon.to_radians());
    [p.cos() * l.cos(), p.cos() * l.sin(), p.sin()]
}
fn tree(ids: &mut [usize], points: &[[f64; 3]], depth: usize) -> Option<Box<Node>> {
    if ids.is_empty() {
        return None;
    }
    let axis = depth % 3;
    let mid = ids.len() / 2;
    ids.select_nth_unstable_by(mid, |a, b| points[*a][axis].total_cmp(&points[*b][axis]));
    let (left, rest) = ids.split_at_mut(mid);
    let (center, right) = rest.split_first_mut()?;
    Some(Box::new(Node {
        index: *center,
        axis,
        left: tree(left, points, depth + 1),
        right: tree(right, points, depth + 1),
    }))
}
fn search(
    node: &Option<Box<Node>>,
    points: &[[f64; 3]],
    target: [f64; 3],
    best: &mut Option<usize>,
    distance: &mut f64,
) {
    let Some(n) = node else {
        return;
    };
    let p = points[n.index];
    let d = (0..3).map(|i| (p[i] - target[i]).powi(2)).sum::<f64>();
    if d <= *distance {
        *distance = d;
        *best = Some(n.index);
    }
    let delta = target[n.axis] - p[n.axis];
    let (near, far) = if delta < 0.0 {
        (&n.left, &n.right)
    } else {
        (&n.right, &n.left)
    };
    search(near, points, target, best, distance);
    if delta * delta <= *distance {
        search(far, points, target, best, distance);
    }
}

/// Distanza in km (formula dell'emisenoverso).
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    6371.0 * 2.0 * a.clamp(0.0, 1.0).sqrt().asin()
}

pub fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            c => c,
        })
        .collect::<String>()
}

impl Gazetteer {
    /// Formato: `id · nome · nome italiano · lat · lon · paese · admin1 · popolazione · fuso IANA`
    /// (TSV; la colonna del fuso è facoltativa).
    pub fn load(path: &Path) -> AppResult<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> AppResult<Self> {
        let mut cities = Vec::new();
        for line in text.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 8 {
                continue;
            }
            let (Ok(lat), Ok(lon)) = (f[3].parse::<f64>(), f[4].parse::<f64>()) else {
                continue;
            };
            if !lat.is_finite()
                || !lon.is_finite()
                || !(-90.0..=90.0).contains(&lat)
                || !(-180.0..=180.0).contains(&lon)
            {
                continue;
            }
            let name = if f[2].is_empty() { f[1] } else { f[2] };
            cities.push(City {
                name: name.to_string(),
                alt_name: f[1].to_string(),
                country: f[5].to_string(),
                lat,
                lon,
                population: f[7].parse().unwrap_or(0),
                timezone: f
                    .get(8)
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            });
        }
        if cities.is_empty() {
            return Err(AppError::new(
                ErrorCode::ModelUnavailable,
                "gazetteer vuoto o non valido",
            ));
        }
        let mut by_name: HashMap<String, usize> = HashMap::new();
        for (i, c) in cities.iter().enumerate() {
            // A parità di nome vince la città più popolosa.
            for key in [fold(&c.name), fold(&c.alt_name)] {
                match by_name.get(&key) {
                    Some(&j) if cities[j].population >= c.population => {}
                    _ => {
                        by_name.insert(key, i);
                    }
                }
            }
        }
        let points: Vec<_> = cities.iter().map(|c| point(c.lat, c.lon)).collect();
        let root = tree(&mut (0..cities.len()).collect::<Vec<_>>(), &points, 0);
        Ok(Self {
            cities,
            points,
            root,
            by_name,
        })
    }

    pub fn len(&self) -> usize {
        self.cities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cities.is_empty()
    }

    /// Città più vicina entro `max_km`.
    pub fn nearest(&self, lat: f64, lon: f64, max_km: f64) -> Option<(&City, f64)> {
        if !lat.is_finite()
            || !lon.is_finite()
            || !max_km.is_finite()
            || max_km < 0.0
            || !(-90.0..=90.0).contains(&lat)
            || !(-180.0..=180.0).contains(&lon)
        {
            return None;
        }
        let mut distance =
            (2.0 * (max_km.min(std::f64::consts::PI * 6371.0) / 12742.0).sin()).powi(2);
        let mut best = None;
        search(
            &self.root,
            &self.points,
            point(lat, lon),
            &mut best,
            &mut distance,
        );
        best.map(|i| {
            (
                &self.cities[i],
                haversine_km(lat, lon, self.cities[i].lat, self.cities[i].lon),
            )
        })
    }

    /// Fuso IANA della città più vicina (senza limite di distanza: è un
    /// suggerimento che l'utente conferma).
    pub fn timezone_at(&self, lat: f64, lon: f64) -> Option<&str> {
        self.nearest(lat, lon, f64::MAX)
            .and_then(|(c, _)| c.timezone.as_deref())
    }

    pub fn by_name(&self, name: &str) -> Option<&City> {
        self.by_name
            .get(&fold(name.trim()))
            .map(|&i| &self.cities[i])
    }
}

/// Nomi di cartella che non indicano un luogo.
fn generic(seg: &str) -> bool {
    let s = fold(seg);
    let words = [
        "dcim",
        "foto",
        "photos",
        "photo",
        "pictures",
        "immagini",
        "camera",
        "camera roll",
        "100apple",
        "download",
        "downloads",
        "desktop",
        "documents",
        "documenti",
        "users",
        "utenti",
        "archivio",
        "backup",
        "nuova cartella",
        "new folder",
        "misc",
        "varie",
        "export",
        "raw",
        "jpg",
    ];
    words.contains(&s.as_str())
        || s.chars().all(|c| c.is_ascii_digit() || "-_ .".contains(c)) // anni, date
        || s.len() <= 2
        || s.ends_with(':')
}

/// Luogo dedotto dal percorso: il segmento più profondo che corrisponde a una
/// città nota; in mancanza, il segmento più profondo non generico.
pub fn place_from_path(path: &Path, gaz: Option<&Gazetteer>) -> Option<String> {
    let full = path.to_string_lossy();
    let (parent, _) = full.rsplit_once(['/', '\\'])?;
    let segs: Vec<String> = parent
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let candidates: Vec<&String> = segs.iter().rev().filter(|s| !generic(s)).collect();
    if let Some(g) = gaz {
        for s in &candidates {
            if let Some(c) = g.by_name(s) {
                return Some(c.name.clone());
            }
        }
    }
    candidates
        .first()
        .map(|s| s.replace(['_', '-'], " ").trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "3176959\tFlorence\tFirenze\t43.77925\t11.24626\tIT\t16\t367150\n\
3166548\tSiena\tSiena\t43.31822\t11.33064\tIT\t16\t53901\n\
4062577\tFlorence\t\t34.79981\t-87.67725\tUS\tAL\t40026\n\
3169070\tRome\tRoma\t41.89193\t12.51133\tIT\t07\t2318895\n";

    #[test]
    fn timezone_of_nearest_city_from_shipped_file() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/models/geo/cities15000.tsv");
        let g = Gazetteer::load(&path).unwrap();
        // Firenze, piazza del Duomo.
        assert_eq!(g.timezone_at(43.7731, 11.2560), Some("Europe/Rome"));
        // Moab (Utah), sotto i 15.000 abitanti: vale la città più vicina.
        assert_eq!(g.timezone_at(38.5733, -109.5498), Some("America/Denver"));
        assert_eq!(g.timezone_at(f64::NAN, 0.0), None);
        // File senza la colonna del fuso: nessun fuso, ma la città si trova.
        let old = Gazetteer::parse(SAMPLE).unwrap();
        assert_eq!(old.timezone_at(43.77, 11.25), None);
    }

    #[test]
    fn nearest_city_with_italian_name() {
        let g = Gazetteer::parse(SAMPLE).unwrap();
        let (c, d) = g.nearest(43.77, 11.25, 30.0).unwrap();
        assert_eq!(c.name, "Firenze");
        assert!(d < 2.0);
        assert!(g.nearest(0.0, 0.0, 30.0).is_none());
        assert_eq!(
            g.by_name("florence").unwrap().country,
            "IT",
            "vince la più popolosa"
        );
    }

    #[test]
    fn place_from_directory_names() {
        let g = Gazetteer::parse(SAMPLE).unwrap();
        let p = Path::new(r"D:\Foto\2023\Toscana\Siena\IMG_1.JPG");
        assert_eq!(place_from_path(p, Some(&g)).as_deref(), Some("Siena"));
        let p = Path::new(r"D:\Foto\2023\Val_d'Orcia\IMG_1.JPG");
        assert_eq!(place_from_path(p, Some(&g)).as_deref(), Some("Val d'Orcia"));
        let p = Path::new(r"D:\Foto\2023\07\DCIM\IMG_1.JPG");
        assert_eq!(place_from_path(p, Some(&g)), None);
        let p = Path::new(r"E:\Archivio\Roma 2019\IMG_1.JPG");
        assert_eq!(place_from_path(p, Some(&g)).as_deref(), Some("Roma 2019"));
    }

    #[test]
    fn kd_tree_handles_antimeridian_and_matches_brute_force() {
        let g = Gazetteer::parse("1\tWest\t\t10\t-179.9\tXX\t0\t1\n2\tEast\t\t10\t170\tXX\t0\t1\n")
            .unwrap();
        assert_eq!(g.nearest(10.0, 179.9, 100.0).unwrap().0.name, "West");
        assert!(g.nearest(f64::NAN, 0.0, 100.0).is_none());
        let mut sample = String::new();
        for i in 0..400 {
            sample.push_str(&format!(
                "{i}\tcity{i}\t\t{}\t{}\tXX\t0\t1\n",
                (i * 17 % 178) as f64 - 89.0,
                (i * 31 % 360) as f64 - 180.0
            ));
        }
        let g = Gazetteer::parse(&sample).unwrap();
        for (lat, lon) in [(89.0, 179.9), (-89.0, -179.9), (43.77, 11.25), (0.0, 0.0)] {
            let brute = g
                .cities
                .iter()
                .map(|c| haversine_km(lat, lon, c.lat, c.lon))
                .fold(f64::INFINITY, f64::min);
            assert!((g.nearest(lat, lon, 20020.0).unwrap().1 - brute).abs() < 1e-6);
        }
    }
}
