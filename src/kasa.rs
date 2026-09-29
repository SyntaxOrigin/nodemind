//! Kasa: not klasörünün özyinelemeli taranması, notların okunması ve
//! **yalnızca yeni** not dosyası oluşturulması.
//!
//! ## Veri güvenliği (rapor b01, b03)
//!
//! NodeMind not dosyalarına **asla yazmaz**. Yalnızca `cache/index.json` ve
//! kullanıcı `graph --cikti` ile adı verdiği dışa aktarım dosyaları oluşturur.
//! `yeni_not` çağrısı, hedef dosya varsa [`Hata::NotZatenVar`] döner ve dosyaya
//! hiç dokunmaz — bu, "var olan notu silme/sıfırla" hatasının kökünü ortadan
//! kaldırır.
//!
//! `walkdir`/`notify` yasak olduğu için gezinme kendi özyinelemeli
//! `std::fs::read_dir` çağrımızla yapılır. Simge bağlantıları (symlink) atlanır;
//! bu, kasa kökünün dışına çıkan bir bağlantının klasörü okumasını engeller.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use crate::gunluk::Tarih;
use crate::hata::Hata;
use crate::kimlik::not_kimligi;
use crate::markdown::{ayristir, Blok, BlokTipi};

/// Taranmayan dizin adları: indeks önbelleği, günlükler ve sürüm kontrolü.
const ATLANAN_DIZINLER: &[&str] = &[
    "cache",
    "logs",
    ".git",
    ".obsidian",
    ".trash",
    "node_modules",
];

/// Windows'un ayrılmış cihaz adları; `CON.md` gibi adlar dosya sisteminde
/// çalışmayabilir, bu yüzden reddedilir.
const AYRILMIS_ADLAR: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Windows dosya adlarında yasak karakterler.
const YASAK_KARAKTERLER: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Bir notun düzleştirilmiş blok özeti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlokOzet {
    /// Blok kimliği.
    pub kimlik: String,
    /// Blok türü.
    pub tip: BlokTipi,
    /// Blok metni.
    pub metin: String,
    /// Not ağacındaki konum yolu.
    pub yol: String,
    /// Başlıktan başlayarak okunabilir yol (`"Giris > Ayrinti"`).
    pub baslik_yolu: String,
    /// Ağaç derinliği.
    pub derinlik: u8,
    /// 1 tabanlı satır numarası.
    pub satir: usize,
    /// Bloğun ait olduğu notun kimliği.
    pub not_id: String,
}

/// Belleğe alınmış tek bir not.
#[derive(Debug, Clone)]
pub struct Not {
    /// Göreli yoldan türetilmiş kararlı kimlik.
    pub id: String,
    /// Kasanın köküne göreli yol (`/` ayraçlı).
    pub yol: String,
    /// Dosya adı (uzantısız).
    pub ad: String,
    /// Görünen başlık: ilk `#` başlık, yoksa dosya adı.
    pub baslik: String,
    /// Dosya adından çözülen günlük tarihi, günlük değilse `None`.
    pub tarih: Option<Tarih>,
    /// Dosyanın son değişiklik zamanı (Unix saniye).
    pub degisiklik: u64,
    /// Blok ağacının kökleri.
    pub bloklar: Vec<Blok>,
    /// Dosyanın bayt cinsinden boyutu.
    pub boyut: u64,
}

impl Not {
    /// Notun bloklarını önce kökler olmak üzere düzleştirir.
    ///
    /// Her blok, kendisine ait başlık zincirini (`baslik_yolu`) taşır; bir
    /// başlığın çocuklarına geçilirken zincir kısalır, başlıktan çıkılırken
    /// geri alınır.
    pub fn duz_bloklar(&self) -> Vec<BlokOzet> {
        let mut cikti = Vec::new();
        for b in &self.bloklar {
            let mut zincir: Vec<String> = Vec::new();
            duz_bloklar_ic(b, &self.id, "", &mut zincir, &mut cikti);
        }
        cikti
    }
}

fn duz_bloklar_ic(
    blok: &Blok,
    not_id: &str,
    ust_baslik: &str,
    baslik_yigini: &mut Vec<String>,
    cikti: &mut Vec<BlokOzet>,
) {
    let baslik_yolu = if blok.tip == BlokTipi::Baslik {
        baslik_yigini.push(blok.metin.clone());
        baslik_yigini.join(" > ")
    } else {
        ust_baslik.to_string()
    };
    cikti.push(BlokOzet {
        kimlik: blok.kimlik.clone(),
        tip: blok.tip,
        metin: blok.metin.clone(),
        yol: blok.yol.clone(),
        baslik_yolu: baslik_yolu.clone(),
        derinlik: blok.derinlik,
        satir: blok.baslangic_satir,
        not_id: not_id.to_string(),
    });
    for cocuk in &blok.cocuklar {
        duz_bloklar_ic(cocuk, not_id, &baslik_yolu, baslik_yigini, cikti);
    }
    if blok.tip == BlokTipi::Baslik {
        baslik_yigini.pop();
    }
}

/// Taranmış not klasörü.
#[derive(Debug, Default, Clone)]
pub struct Kasa {
    /// Kasanın kök dizini.
    pub kok: PathBuf,
    /// Notlar (göreli yola göre sıralı).
    pub notlar: Vec<Not>,
    /// Tarama sırasında atlanan yollar ve nedenleri.
    pub uyarilar: Vec<String>,
}

impl Kasa {
    /// Verilen dizini tarar ve boş olmayan notlardan oluşan bir kasa kurar.
    ///
    /// Hiç `.md` dosyası bulunmayan bir klasör hata **değildir**; boş kasa döner
    /// ve `graph` boş grafiği yazar. Bu, "ilk çalıştırmada boş klasör" senaryosunu
    /// kırmaz.
    pub fn ac(kok: &Path) -> Result<Kasa, Hata> {
        if !kok.is_dir() {
            if kok.exists() {
                return Err(Hata::KlasorDegil {
                    yol: kok.to_path_buf(),
                });
            }
            return Err(Hata::KlasorDegil {
                yol: kok.to_path_buf(),
            });
        }
        let mut kasa = Kasa {
            kok: kok.to_path_buf(),
            notlar: Vec::new(),
            uyarilar: Vec::new(),
        };
        kasa.tara(kok)?;
        kasa.notlar.sort_by(|a, b| a.yol.cmp(&b.yol));
        kasa.notlar.dedup_by(|a, b| a.yol == b.yol);
        Ok(kasa)
    }

    fn tara(&mut self, dizin: &Path) -> Result<(), Hata> {
        let girdiler =
            std::fs::read_dir(dizin).map_err(|e| crate::hata::io_hata("dizin okuma", dizin, e))?;
        for giris in girdiler {
            let giris = match giris {
                Ok(g) => g,
                Err(e) => {
                    self.uyarilar.push(format!(
                        "{}: dizin girdisi okunamadi ({e})",
                        dizin.display()
                    ));
                    continue;
                }
            };
            let yol = giris.path();
            let ad = giris.file_name().to_string_lossy().to_string();
            // Simge bağlantıları atlanır: kasa disina cikis ve dongu dongusu engellenir.
            if giris.file_type().map(|t| t.is_symlink()).unwrap_or(false) {
                self.uyarilar
                    .push(format!("{ad}: simge baglantisi atlandi"));
                continue;
            }
            if yol.is_dir() {
                if ad.starts_with('.') || ATLANAN_DIZINLER.contains(&ad.as_str()) {
                    continue;
                }
                self.tara(&yol)?;
                continue;
            }
            let kucuk = ad.to_ascii_lowercase();
            if !(kucuk.ends_with(".md") || kucuk.ends_with(".markdown")) {
                continue;
            }
            match self.not_oku(dizin, &yol) {
                Ok(not) => self.notlar.push(not),
                Err(e) => self.uyarilar.push(format!("{}: {e}", yol.display())),
            }
        }
        Ok(())
    }

    fn not_oku(&mut self, ust: &Path, yol: &Path) -> Result<Not, Hata> {
        let veri = std::fs::read(yol).map_err(|e| crate::hata::io_hata("not okuma", yol, e))?;
        let boyut = veri.len() as u64;
        let metin = String::from_utf8_lossy(&veri).to_string();
        let goreli = ust
            .strip_prefix(&self.kok)
            .unwrap_or(ust)
            .join(yol.file_name().unwrap_or_default());
        let goreli = goreli.to_string_lossy().replace('\\', "/");
        let id = not_kimligi(&goreli);
        let ad = yol
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let tarih = Tarih::dosya_adindan(&ad);
        let mut bloklar = ayristir(&metin, &id);
        // Boş dosya: indekslenebilir ama blok üretmez; bu, "boş not" davranışıdır.
        if bloklar.is_empty() {
            bloklar = Vec::new();
        }
        let baslik = not_basligi(&bloklar, &ad);
        let degisiklik = std::fs::metadata(yol)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok(Not {
            id,
            yol: goreli,
            ad,
            baslik,
            tarih,
            degisiklik,
            bloklar,
            boyut,
        })
    }

    /// Toplam not ve blok sayısını döndürür.
    ///
    /// Blok sayısı **kök değil tüm** blokları kapsar: iç içe liste maddeleri ve
    /// başlık altı paragraflar da sayılır.
    pub fn olcek(&self) -> (usize, usize) {
        let blok = self.notlar.iter().map(|n| n.duz_bloklar().len()).sum();
        (self.notlar.len(), blok)
    }

    /// Not adı ya da göreli yol ile eşleşen notları döndürür.
    ///
    /// Karşılaştırma ASCII'ye katlanmış ve büyük/küçük harf duyarsızdır; yol
    /// eşleşmesi `a/b.md` ve `b.md` biçimlerini de kabul eder.
    pub fn not_ara(&self, secim: &str) -> Vec<&Not> {
        let hedef = crate::indeks::katla(secim);
        let hedef_yol = crate::indeks::katla(&secim.replace('\\', "/"));
        let mut sonuc: Vec<&Not> = self
            .notlar
            .iter()
            .filter(|n| {
                let ad = crate::indeks::katla(&n.ad);
                ad == hedef
                    || crate::indeks::katla(&n.yol) == hedef_yol
                    || n.yol.eq_ignore_ascii_case(secim)
            })
            .collect();
        sonuc.sort_by(|a, b| a.yol.cmp(&b.yol));
        sonuc
    }

    /// Blok kimliğiyle eşleşen bloğu ve ait olduğu notu döndürür.
    pub fn blok_ara(&self, kimlik: &str) -> Option<(&Not, BlokOzet)> {
        for not in &self.notlar {
            for ozet in not.duz_bloklar() {
                if ozet.kimlik == kimlik {
                    return Some((not, ozet));
                }
            }
        }
        None
    }

    /// Aynı ada sahip not gruplarını döndürür (belirsizlik denetimi).
    ///
    /// Gruplama **katlanmış** ada göre yapılır (`Çalışma` = `calisma`), ama
    /// kullanıcıya katlanmış değil **özgün** ad gösterilir: `2026 09 01`
    /// yerine `2026-09-01` yazılmalıdır.
    pub fn yinelenen_adlar(&self) -> Vec<(String, Vec<String>)> {
        let mut sirali: Vec<(String, BTreeSet<String>)> = Vec::new();
        let mut indeks: BTreeMap<String, usize> = BTreeMap::new();
        for n in &self.notlar {
            let anahtar = crate::indeks::katla(&n.ad);
            match indeks.get(&anahtar) {
                Some(i) => {
                    if let Some(g) = sirali.get_mut(*i) {
                        g.0 = n.ad.clone();
                        g.1.insert(n.yol.clone());
                    }
                }
                None => {
                    indeks.insert(anahtar, sirali.len());
                    sirali.push((n.ad.clone(), BTreeSet::from([n.yol.clone()])));
                }
            }
        }
        sirali
            .into_iter()
            .filter(|(_, yollar)| yollar.len() > 1)
            .map(|(ad, yollar)| (ad, yollar.into_iter().collect()))
            .collect()
    }

    /// Kasanın `cache/index.json` yolunu döndürür.
    pub fn indeks_yolu(&self) -> PathBuf {
        self.kok
            .join(crate::sema::CACHE_DIZINI)
            .join(crate::sema::INDEKS_DOSYASI)
    }

    /// **Yeni** bir not dosyası oluşturur.
    ///
    /// Var olan bir dosyaya **hiçbir koşulda yazmaz**: dosya varsa
    /// [`Hata::NotZatenVar`] döner. Bu, veri güvenliği kuralının uygulama
    /// noktasıdır.
    pub fn yeni_not(&self, ad: &str, icerik: &str) -> Result<PathBuf, Hata> {
        ad_dogrula(ad)?;
        let yol = self.kok.join(format!("{ad}.md"));
        if yol.exists() {
            return Err(Hata::NotZatenVar { yol });
        }
        if let Some(ust) = yol.parent() {
            if !ust.starts_with(&self.kok) {
                return Err(Hata::KasaDisiYol {
                    yol: ad.to_string(),
                });
            }
        }
        std::fs::write(&yol, icerik).map_err(|e| crate::hata::io_hata("not olusturma", &yol, e))?;
        Ok(yol)
    }
}

/// Notun görünen başlığını belirler: ilk `#` başlık, yoksa dosya adı.
fn not_basligi(bloklar: &[Blok], ad: &str) -> String {
    crate::markdown::duzlestir(bloklar)
        .into_iter()
        .find(|b| b.tip == BlokTipi::Baslik && !b.metin.is_empty())
        .map(|b| b.metin.clone())
        .unwrap_or_else(|| ad.to_string())
}

/// Not adının güvenli olup olmadığını denetler.
///
/// Reddedilen durumlar: boş ad, yol ayracı, `.`/`..`, ayrılmış Windows adları,
/// yasak karakterler, baştaki/sondaki boşluk veya nokta.
pub fn ad_dogrula(ad: &str) -> Result<(), Hata> {
    let kirp = ad.trim();
    let red = |sebep: &'static str| Hata::GecersizNotAdi {
        ad: ad.to_string(),
        sebep,
    };
    if ad.is_empty() {
        return Err(red("ad bos olamaz"));
    }
    if kirp != ad {
        return Err(red("bastaki veya sondaki bosluk gecersiz"));
    }
    if ad == "." || ad == ".." {
        return Err(red("nokta adlari gecersiz"));
    }
    if ad.contains('/') || ad.contains('\\') {
        return Err(red("yol ayraci iceriyor; alt klasor kullanilamaz"));
    }
    if ad.contains("..") {
        return Err(red("iki nokta iceriyor; yol kacisi denemesi"));
    }
    if ad.chars().any(|c| YASAK_KARAKTERLER.contains(&c)) {
        return Err(red("yasak dosya adi karakteri iceriyor"));
    }
    if ad.chars().any(|c| (c as u32) < 0x20) {
        return Err(red("kontrol karakteri iceriyor"));
    }
    if ad.starts_with('.') || ad.ends_with('.') {
        return Err(red("nokta ile baslamaz veya bitmez"));
    }
    if ad.ends_with(' ') {
        return Err(red("bosluk ile bitmez"));
    }
    let kok = ad.split('.').next().unwrap_or(ad).to_ascii_uppercase();
    if AYRILMIS_ADLAR.contains(&kok.as_str()) {
        return Err(red("Windows ayrilmis cihaz adi"));
    }
    if ad.len() > 120 {
        return Err(red("ad 120 karakterden uzun olamaz"));
    }
    Ok(())
}

/// Göreli bir yolun kasa kökünün dışına çıkıp çıkmadığını denetler.
///
/// `graph --cikti ../kacis.dot` gibi bir komutun kasa dışına yazmasını engeller.
pub fn kasa_ici_mi(kok: &Path, yol: &Path) -> bool {
    if yol.is_absolute() {
        return yol.starts_with(kok);
    }
    let mut toplam = PathBuf::from(kok);
    for b in yol.components() {
        match b {
            Component::CurDir => {}
            Component::ParentDir => {
                if !toplam.pop() {
                    return false;
                }
            }
            Component::Normal(ad) => toplam.push(ad),
            _ => return false,
        }
    }
    toplam.starts_with(kok)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::test_yardimcisi::GeciciDizin;

    fn yaz(d: &Path, goreli: &str, icerik: &str) {
        let yol = d.join(goreli);
        if let Some(ust) = yol.parent() {
            std::fs::create_dir_all(ust).unwrap();
        }
        std::fs::write(yol, icerik).unwrap();
    }

    // ---- ad dogrulama ----

    #[test]
    fn gecerli_ad_kabul_edilir() {
        for ad in ["not", "2026-09-29", "Markdown Lehcesi", "not_1", "a.b"] {
            assert!(ad_dogrula(ad).is_ok(), "{ad} kabul edilmeli");
        }
    }

    #[test]
    fn bos_ad_reddedilir() {
        assert!(ad_dogrula("").is_err());
    }

    #[test]
    fn yol_kacisi_reddedilir() {
        for ad in ["../kacis", "..", ".", "a/b", "a\\b", "x/../y", "..\\kacis"] {
            assert!(ad_dogrula(ad).is_err(), "{ad} reddedilmeli");
        }
    }

    #[test]
    fn yasak_karakterler_reddedilir() {
        for ad in ["a:b", "a?b", "a*b", "a<b", "a>b", "a|b", "a\"b"] {
            assert!(ad_dogrula(ad).is_err(), "{ad} reddedilmeli");
        }
    }

    #[test]
    fn ayrilmis_cihaz_adlari_reddedilir() {
        for ad in ["CON", "con", "NUL", "COM1", "LPT9", "nul.md"] {
            assert!(ad_dogrula(ad).is_err(), "{ad} reddedilmeli");
        }
    }

    #[test]
    fn kontrol_karakteri_ve_kenar_boasluklari_reddedilir() {
        assert!(ad_dogrula("a\u{1}b").is_err());
        assert!(ad_dogrula(" once").is_err());
        assert!(ad_dogrula("son ").is_err());
        assert!(ad_dogrula(".gizli").is_err());
        assert!(ad_dogrula("son.").is_err());
    }

    #[test]
    fn cok_uzun_ad_reddedilir() {
        assert!(ad_dogrula(&"a".repeat(121)).is_err());
    }

    // ---- kasa ici mi ----

    #[test]
    fn kasa_ici_yollar_kabul_edilir() {
        let kok = Path::new("/kasa");
        assert!(kasa_ici_mi(kok, Path::new("a.md")));
        assert!(kasa_ici_mi(kok, Path::new("alt/a.md")));
        assert!(kasa_ici_mi(kok, Path::new("./alt/../a.md")));
    }

    #[test]
    fn kasa_disi_yollar_reddedilir() {
        let kok = Path::new("/kasa");
        assert!(!kasa_ici_mi(kok, Path::new("../a.md")));
        assert!(!kasa_ici_mi(kok, Path::new("alt/../../a.md")));
        assert!(!kasa_ici_mi(kok, Path::new("/diger/a.md")));
    }

    // ---- tarama ----

    #[test]
    fn olmayan_klasor_hata_doner() {
        let d = GeciciDizin::yeni("nodemind-kasa-yok").unwrap();
        assert!(Kasa::ac(&d.yol().join("yok")).is_err());
    }

    #[test]
    fn dosya_verilen_yol_hata_doner() {
        let d = GeciciDizin::yeni("nodemind-kasa-dosya").unwrap();
        yaz(d.yol(), "a.md", "# A");
        assert!(Kasa::ac(&d.yol().join("a.md")).is_err());
    }

    #[test]
    fn bos_klasor_bos_kasa_dondurur() {
        let d = GeciciDizin::yeni("nodemind-kasa-bos").unwrap();
        let k = Kasa::ac(d.yol()).unwrap();
        assert_eq!(k.notlar.len(), 0);
        assert_eq!(k.olcek(), (0, 0));
    }

    #[test]
    fn markdown_disi_dosyalar_yoksayilir() {
        let d = GeciciDizin::yeni("nodemind-kasa-uzanti").unwrap();
        yaz(d.yol(), "a.md", "# A");
        yaz(d.yol(), "b.txt", "duz metin");
        yaz(d.yol(), "c.markdown", "# C");
        let k = Kasa::ac(d.yol()).unwrap();
        assert_eq!(k.notlar.len(), 2);
    }

    #[test]
    fn alt_klasorler_ozyinelemeli_taranir() {
        let d = GeciciDizin::yeni("nodemind-kasa-alt").unwrap();
        yaz(d.yol(), "ust.md", "# Ust");
        yaz(d.yol(), "gunluk/2026-09-29.md", "# Gunluk");
        yaz(d.yol(), "gunluk/alt/derin.md", "# Derin");
        let k = Kasa::ac(d.yol()).unwrap();
        assert_eq!(k.notlar.len(), 3);
        assert!(k.notlar.iter().any(|n| n.yol == "gunluk/alt/derin.md"));
    }

    #[test]
    fn cache_logs_ve_gizli_dizinler_atlanir() {
        let d = GeciciDizin::yeni("nodemind-kasa-atla").unwrap();
        yaz(d.yol(), "a.md", "# A");
        yaz(d.yol(), "cache/index.json", "{}");
        yaz(d.yol(), "cache/i.md", "# Cache");
        yaz(d.yol(), "logs/x.log", "kayit");
        yaz(d.yol(), ".git/config", "x");
        yaz(d.yol(), ".gizli/g.md", "# Gizli");
        let k = Kasa::ac(d.yol()).unwrap();
        assert_eq!(k.notlar.len(), 1);
    }

    #[test]
    fn not_bilgileri_dogru_doldurulur() {
        let d = GeciciDizin::yeni("nodemind-kasa-not").unwrap();
        yaz(
            d.yol(),
            "gunluk/2026-09-29.md",
            "# 29 Eylul\n\n- bir\n- iki\n",
        );
        let k = Kasa::ac(d.yol()).unwrap();
        let n = &k.notlar[0];
        assert_eq!(n.ad, "2026-09-29");
        assert_eq!(n.baslik, "29 Eylul");
        assert_eq!(n.yol, "gunluk/2026-09-29.md");
        assert!(n.tarih.is_some());
        assert_eq!(n.tarih.unwrap().iso_metni(), "2026-09-29");
        assert_eq!(n.duz_bloklar().len(), 3);
        assert!(n.boyut > 0);
    }

    #[test]
    fn basligiz_not_dosya_adini_kullanir() {
        let d = GeciciDizin::yeni("nodemind-kasa-basliksiz").unwrap();
        yaz(d.yol(), "adsiz.md", "sadece paragraf");
        let k = Kasa::ac(d.yol()).unwrap();
        assert_eq!(k.notlar[0].baslik, "adsiz");
    }

    #[test]
    fn gunluk_disi_ad_tarih_uretmez() {
        let d = GeciciDizin::yeni("nodemind-kasa-tarih").unwrap();
        yaz(d.yol(), "not-2026-13-45.md", "# x");
        let k = Kasa::ac(d.yol()).unwrap();
        assert!(k.notlar[0].tarih.is_none());
    }

    #[test]
    fn blok_ozeti_baslik_yolunu_biriktirir() {
        let d = GeciciDizin::yeni("nodemind-kasa-yol").unwrap();
        yaz(d.yol(), "a.md", "# A\n## B\n### C\n- madde\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let ozetler = k.notlar[0].duz_bloklar();
        assert_eq!(ozetler[0].baslik_yolu, "A");
        assert_eq!(ozetler[1].baslik_yolu, "A > B");
        assert_eq!(ozetler[2].baslik_yolu, "A > B > C");
        assert_eq!(ozetler[3].baslik_yolu, "A > B > C");
        assert_eq!(ozetler[3].tip, BlokTipi::Liste);
    }

    #[test]
    fn bos_not_dosyasi_taranir_ama_blok_uretmez() {
        let d = GeciciDizin::yeni("nodemind-kasa-bosnot").unwrap();
        yaz(d.yol(), "bos.md", "");
        let k = Kasa::ac(d.yol()).unwrap();
        assert_eq!(k.notlar.len(), 1);
        assert!(k.notlar[0].duz_bloklar().is_empty());
        assert_eq!(k.notlar[0].baslik, "bos");
    }

    #[test]
    fn not_ara_ada_ve_yolla_calisir() {
        let d = GeciciDizin::yeni("nodemind-kasa-ara").unwrap();
        yaz(d.yol(), "Markdown Lehcesi.md", "# ML");
        yaz(d.yol(), "alt/derin.md", "# D");
        let k = Kasa::ac(d.yol()).unwrap();
        assert_eq!(k.not_ara("Markdown Lehcesi").len(), 1);
        assert_eq!(k.not_ara("markdown lehcesi").len(), 1);
        assert_eq!(k.not_ara("alt/derin.md").len(), 1);
        assert_eq!(k.not_ara("derin").len(), 1);
        assert_eq!(k.not_ara("yok").len(), 0);
    }

    #[test]
    fn blok_ara_kimlikle_calisir() {
        let d = GeciciDizin::yeni("nodemind-kasa-blokara").unwrap();
        yaz(d.yol(), "a.md", "# A");
        let k = Kasa::ac(d.yol()).unwrap();
        let kimlik = k.notlar[0].duz_bloklar()[0].kimlik.clone();
        let (n, o) = k.blok_ara(&kimlik).unwrap();
        assert_eq!(n.ad, "a");
        assert_eq!(o.metin, "A");
        assert!(k.blok_ara("yok").is_none());
    }

    #[test]
    fn yinelenen_adlar_bulunur() {
        let d = GeciciDizin::yeni("nodemind-kasa-yinelenen").unwrap();
        yaz(d.yol(), "bir/not.md", "# 1");
        yaz(d.yol(), "iki/not.md", "# 2");
        yaz(d.yol(), "tekil.md", "# 3");
        let k = Kasa::ac(d.yol()).unwrap();
        let y = k.yinelenen_adlar();
        assert_eq!(y.len(), 1);
        assert_eq!(y[0].0, "not", "katlanmis degil ozgun ad yazilmali");
        assert_eq!(y[0].1.len(), 2);
    }

    #[test]
    fn indeks_yolu_cache_altindadir() {
        let d = GeciciDizin::yeni("nodemind-kasa-indeksyolu").unwrap();
        let k = Kasa::ac(d.yol()).unwrap();
        assert!(k.indeks_yolu().ends_with("cache/index.json"));
    }

    // ---- yeni not (veri guvenligi) ----

    #[test]
    fn yeni_not_olusturur_ve_okunabilir() {
        let d = GeciciDizin::yeni("nodemind-kasa-yeni").unwrap();
        let k = Kasa::ac(d.yol()).unwrap();
        let yol = k.yeni_not("t Not", "# t Not\n").unwrap();
        assert!(yol.exists());
        let icerik = std::fs::read_to_string(&yol).unwrap();
        assert_eq!(icerik, "# t Not\n");
    }

    #[test]
    fn var_olan_not_uzerine_yazilmaz() {
        let d = GeciciDizin::yeni("nodemind-kasa-var").unwrap();
        yaz(d.yol(), "var.md", "# Orijinal icerik");
        let k = Kasa::ac(d.yol()).unwrap();
        let h = k.yeni_not("var", "# YENI\n").unwrap_err();
        assert!(matches!(h, Hata::NotZatenVar { .. }));
        let icerik = std::fs::read_to_string(d.yol().join("var.md")).unwrap();
        assert_eq!(icerik, "# Orijinal icerik");
    }

    #[test]
    fn gecersiz_adla_yeni_not_olusturulmaz() {
        let d = GeciciDizin::yeni("nodemind-kasa-gecersiz").unwrap();
        let k = Kasa::ac(d.yol()).unwrap();
        assert!(k.yeni_not("../kacis", "x").is_err());
        assert!(!d.yol().parent().unwrap().join("kacis.md").exists());
    }

    #[test]
    fn not_olusturma_klasor_dizinine_dokunmaz() {
        let d = GeciciDizin::yeni("nodemind-kasa-dokunma").unwrap();
        yaz(d.yol(), "a.md", "# A");
        let k = Kasa::ac(d.yol()).unwrap();
        let _ = k.yeni_not("b", "# B");
        let k2 = Kasa::ac(d.yol()).unwrap();
        assert_eq!(k2.not_ara("a")[0].duz_bloklar().len(), 1);
    }
}
