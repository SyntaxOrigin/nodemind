//! Bilgi grafiği: düğüm/kenar modeli, terminal ASCII çizimi ve dışa aktarımlar.
//!
//! Grafik kütüphanesi **yasaktır** (`WORKER_CONTRACT.md` § 3.2-G): pencere
//! vektörü, WebView ve SVG yazan crate'ler yerine elle üretilmiş ASCII çizim,
//! Graphviz DOT ve bağımsız (JavaScript'siz) bir HTML/SVG sayfası üretilir.
//!
//! Çizim modeli: grafin **yönsüz** bileşenleri BFS ile bulunur, her bileşen
//! en küçük düğüm kimliğinden kök alınarak bir **genişlik öncelikli ağaç**
//! olarak çizilir. Ağaç kenarı olmayan kenarlar (yani geri ve ileri kenarlar)
//! bileşen başlığının altında `cevrim:` satırında listelenir; böylece döngüler
//! **sessizce kaybolmaz** ve kopuk bileşenler ayrı ayrı görünür.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::hata::Hata;
use crate::kasa::Kasa;
use crate::markdown::{duzlestir, Blok, BlokTipi};
use crate::referans::{baglantilari_cikar, coz, BaglantiTipi, Cozum, Duzce};

/// Bir düğümün türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DugumTipi {
    /// Bir not dosyası.
    Not,
    /// Başlık bloğu.
    Baslik,
    /// Liste maddesi.
    Madde,
    /// Alıntı bloğu.
    Alinti,
    /// Kod bloğu.
    Kod,
    /// Paragraf.
    Paragraf,
}

impl DugumTipi {
    /// Türün kısa etiketi (ASCII çizimde kullanılır).
    pub fn etiket(self) -> &'static str {
        match self {
            DugumTipi::Not => "not",
            DugumTipi::Baslik => "baslik",
            DugumTipi::Madde => "madde",
            DugumTipi::Alinti => "alinti",
            DugumTipi::Kod => "kod",
            DugumTipi::Paragraf => "paragraf",
        }
    }

    /// Blok türünden düğüm türüne dönüşüm.
    pub fn bloktan(tip: BlokTipi) -> Self {
        match tip {
            BlokTipi::Baslik => DugumTipi::Baslik,
            BlokTipi::Liste => DugumTipi::Madde,
            BlokTipi::Alinti => DugumTipi::Alinti,
            BlokTipi::Kod => DugumTipi::Kod,
            BlokTipi::Paragraf => DugumTipi::Paragraf,
        }
    }
}

/// Bir kenarın türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KenarTipi {
    /// Not → kök blok (yapı).
    Yapi,
    /// Ebeveyn blok → çocuk blok (yapı).
    Ice,
    /// Bağlantı kenarı: kaynak blok → hedef not veya blok.
    Baglanti,
}

impl KenarTipi {
    /// Türün kısa etiketi.
    pub fn etiket(self) -> &'static str {
        match self {
            KenarTipi::Yapi => "yapi",
            KenarTipi::Ice => "ice",
            KenarTipi::Baglanti => "baglanti",
        }
    }
}

/// Graf düğümü.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dugum {
    /// Düğüm kimliği (not kimliği veya blok kimliği).
    pub id: String,
    /// Görünen etiket (kısaltılmış).
    pub etiket: String,
    /// Düğüm türü.
    pub tip: DugumTipi,
    /// Not düğümü için: bu notun kök blokları arasındaki bağlantı sayısı.
    pub cikis: usize,
    /// Not düğümü için: bu nota gelen bağlantı sayısı (derece).
    pub giris: usize,
}

/// Graf kenarı.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Kenar {
    /// Kaynak düğüm kimliği.
    pub kaynak: String,
    /// Hedef düğüm kimliği.
    pub hedef: String,
    /// Kenar türü.
    pub tip: KenarTipi,
}

/// Bilgi grafiğinin tamamı.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Graf {
    /// Düğümler (kimliğe göre sıralı).
    pub dugumler: Vec<Dugum>,
    /// Kenarlar (kaynak, hedef, türe göre sıralı).
    pub kenarlar: Vec<Kenar>,
}

/// Hangi düğümlerin grafa gireceğini belirleyen süzgeç.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DugumSecimi {
    /// Yalnızca not düğümleri.
    Not,
    /// Notlar ve başlıklar.
    NotBaslik,
    /// Her blok (varsayılan).
    Hepsi,
}

impl DugumSecimi {
    /// Bir blok türü seçime dahil mi?
    pub fn kapsar(self, tip: BlokTipi) -> bool {
        match self {
            DugumSecimi::Not => false,
            DugumSecimi::NotBaslik => tip == BlokTipi::Baslik,
            DugumSecimi::Hepsi => true,
        }
    }
}

/// Etiketleri ASCII'ye katlar ve verilen uzunlukta kısaltır.
fn kisa(etiket: &str, adet: usize) -> String {
    let duz: String = etiket.split_whitespace().collect::<Vec<_>>().join(" ");
    if duz.chars().count() <= adet {
        return duz;
    }
    let ilk = adet.saturating_sub(3);
    let mut s: String = duz.chars().take(ilk).collect();
    s.push_str("...");
    s
}

impl Graf {
    /// Kasadan graf üretir.
    ///
    /// Yapı kenarları blok ağacından, bağlantı kenarları `[[...]]` ve
    /// `((...))` işaretçilerinden türetilir. **Kırık ve belirsiz** bağlantılar
    /// kenara dönüşmez: bunlar `check` alt komutunun işidir, graf yalnızca
    /// çözülmüş bağlantıları gösterir.
    pub fn olustur(kasa: &Kasa, secim: DugumSecimi) -> Graf {
        let mut dugumler: BTreeMap<String, Dugum> = BTreeMap::new();
        let mut kenarlar: BTreeSet<(String, String, KenarTipi)> = BTreeSet::new();
        let duzce = crate::cozum::duzce_olustur(kasa);

        for not in &kasa.notlar {
            dugumler.insert(
                not.id.clone(),
                Dugum {
                    id: not.id.clone(),
                    etiket: kisa(&not.baslik, 40),
                    tip: DugumTipi::Not,
                    cikis: 0,
                    giris: 0,
                },
            );
            for kok in &not.bloklar {
                dugum_ekle(kok, secim, &mut dugumler);
                if secim.kapsar(kok.tip) {
                    kenarlar.insert((not.id.clone(), kok.kimlik.clone(), KenarTipi::Yapi));
                }
                agac_kenarlar(kok, secim, &mut kenarlar);
            }
        }

        // Bağlantı kenarları: kırık ve belirsiz bağlantılar dışarıda kalır.
        //
        // Derece (giriş/çıkış sayısı) **filtre bağımsızdır**: bağlantı kaynağı
        // bir liste maddesi olsa da `graph --dugum not` seçildiğinde o düğüm
        // çizimde yoktur; yine de "bu nota kaç bağlantı geliyor" sorusunun
        // cevabı değişmemelidir.
        for not in &kasa.notlar {
            for blok in duzlestir(&not.bloklar) {
                for baglanti in baglantilari_cikar(&blok.metin) {
                    let c = coz(&baglanti, &not.id, &blok.kimlik, &duzce);
                    if c.kirik_mi() || c.belirsiz_mi() {
                        continue;
                    }
                    let hedef_dugum = hedef_dugum(&c, &duzce);
                    // Dereçe **not** üzerinden sayılır: `((blok))` bağlantısında
                    // hedef bir bloktur ama "kaç bağlantı geldi" sorusu notu
                    // ilgilendirir. `[[ad]]` bağlantısında hedef zaten nottur.
                    let hedef_not = match c.tip {
                        BaglantiTipi::NotAdi => hedef_dugum.clone(),
                        BaglantiTipi::BlokKimligi => hedef_dugum
                            .as_deref()
                            .and_then(|h| duzce.blok_ara(h))
                            .map(|k| k.not_id.clone()),
                    };
                    if let Some(d) = dugumler.get_mut(&not.id) {
                        d.cikis += 1;
                    }
                    if let Some(hn) = &hedef_not {
                        if let Some(d) = dugumler.get_mut(hn) {
                            d.giris += 1;
                        }
                    }
                    if let Some(hedef) = hedef_dugum {
                        if dugumler.contains_key(&blok.kimlik) && dugumler.contains_key(&hedef) {
                            kenarlar.insert((blok.kimlik.clone(), hedef, KenarTipi::Baglanti));
                        }
                    }
                }
            }
        }

        let mut kenarlar: Vec<Kenar> = kenarlar
            .into_iter()
            .map(|(kaynak, hedef, tip)| Kenar { kaynak, hedef, tip })
            .collect();

        kenarlar.sort();
        Graf {
            dugumler: dugumler.into_values().collect(),
            kenarlar,
        }
    }

    /// Düğüm sayısı.
    pub fn dugum_sayisi(&self) -> usize {
        self.dugumler.len()
    }

    /// Kenar sayısı.
    pub fn kenar_sayisi(&self) -> usize {
        self.kenarlar.len()
    }

    /// Yönsüz komşuluk listesi.
    fn komsular(&self) -> BTreeMap<&str, Vec<&str>> {
        let mut m: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for d in &self.dugumler {
            m.entry(d.id.as_str()).or_default();
        }
        for k in &self.kenarlar {
            m.entry(k.kaynak.as_str())
                .or_default()
                .push(k.hedef.as_str());
            m.entry(k.hedef.as_str())
                .or_default()
                .push(k.kaynak.as_str());
        }
        for v in m.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
        m
    }

    /// Yönsüz bağlantılı bileşenleri, her biri kökten başlayarak BFS sırasıyla
    /// döndürür.
    ///
    /// Bileşen kökü **not düğümleri arasından** seçilir: not, grafiğin doğal
    /// giriş noktasıdır ve ASCII çizimde okunabilir bir ağaç verir. Not
    /// düğümü yoksa (ör. yalnız bloklardan oluşan bir bileşen) en küçük kimlik
    /// kök alınır; seçim **her zaman aynıdır** (deterministik).
    pub fn bilesenler(&self) -> Vec<Vec<String>> {
        let komsu = self.komsular();
        let mut ziyaret: BTreeSet<String> = BTreeSet::new();
        let mut sirali: Vec<&Dugum> = self.dugumler.iter().collect();
        sirali.sort_by(|a, b| a.tip.cmp(&b.tip).then_with(|| a.id.cmp(&b.id)));
        let mut cikti = Vec::new();
        for d in sirali {
            if ziyaret.contains(&d.id) {
                continue;
            }
            let mut sira: VecDeque<String> = VecDeque::new();
            let mut bilesen: Vec<String> = Vec::new();
            sira.push_back(d.id.clone());
            ziyaret.insert(d.id.clone());
            while let Some(simdi) = sira.pop_front() {
                bilesen.push(simdi.clone());
                if let Some(komsular) = komsu.get(simdi.as_str()) {
                    for k in komsular {
                        if ziyaret.insert((*k).to_string()) {
                            sira.push_back((*k).to_string());
                        }
                    }
                }
            }
            cikti.push(bilesen);
        }
        cikti
    }

    /// Ağaç kenarı olmayan kenarları (yani döngüleri) döndürür.
    ///
    /// Her bileşen için BFS kapsayan ağacı kurulur; ağaçta olmayan her kenar
    /// döngü kenarıdır ve bir kez listelenir.
    pub fn cevrim_kenarlari(&self) -> Vec<Kenar> {
        let mut agac: BTreeSet<(String, String)> = BTreeSet::new();
        let komsu = self.komsular();
        for bilesen in self.bilesenler() {
            let kok = bilesen[0].clone();
            let mut ziyaret: BTreeSet<String> = BTreeSet::new();
            let mut sira: VecDeque<String> = VecDeque::new();
            ziyaret.insert(kok.clone());
            sira.push_back(kok);
            while let Some(simdi) = sira.pop_front() {
                if let Some(komsular) = komsu.get(simdi.as_str()) {
                    for k in komsular {
                        if ziyaret.insert((*k).to_string()) {
                            agac.insert((simdi.clone(), (*k).to_string()));
                            sira.push_back((*k).to_string());
                        }
                    }
                }
            }
        }
        self.kenarlar
            .iter()
            .filter(|k| !agac.contains(&(k.kaynak.clone(), k.hedef.clone())))
            .cloned()
            .collect()
    }

    /// Terminal ASCII çizimi.
    ///
    /// Düğümsüz graf, tek düğümlü graf, döngülü graf ve kopuk bileşenler
    /// ayrı ayrı ele alınır; çizim hiçbir durumda boş satır üretmez.
    pub fn ascii(&self) -> String {
        let bilesenler = self.bilesenler();
        let cevrimler = self.cevrim_kenarlari();
        let mut cikti = String::new();
        cikti.push_str(&format!(
            "bilgi grafigi: {} dugum, {} kenar, {} bilesen, {} cevrim kenar\n",
            self.dugum_sayisi(),
            self.kenar_sayisi(),
            bilesenler.len(),
            cevrimler.len()
        ));
        if self.dugumler.is_empty() {
            cikti.push_str("graf bos: kasa icinde hicbir Markdown notu bulunamadi\n");
            return cikti;
        }
        let harita: BTreeMap<&str, &Dugum> =
            self.dugumler.iter().map(|d| (d.id.as_str(), d)).collect();
        for (i, bilesen) in bilesenler.iter().enumerate() {
            let ic_kenarlar = self.bilesen_kenarlari(bilesen);
            let ic_cevrim = cevrimler
                .iter()
                .filter(|k| bilesen.contains(&k.kaynak))
                .count();
            cikti.push('\n');
            cikti.push_str(&format!(
                "bilesen {} - {} dugum, {} kenar",
                i + 1,
                bilesen.len(),
                ic_kenarlar.len()
            ));
            if ic_cevrim > 0 {
                cikti.push_str(&format!(", {ic_cevrim} cevrim"));
            }
            cikti.push('\n');
            agac_ciz(&mut cikti, bilesen, &ic_kenarlar, &harita);
            for k in cevrimler.iter().filter(|k| bilesen.contains(&k.kaynak)) {
                let kaynak = harita
                    .get(k.kaynak.as_str())
                    .map(|d| d.etiket.as_str())
                    .unwrap_or("(yok)");
                let hedef = harita
                    .get(k.hedef.as_str())
                    .map(|d| d.etiket.as_str())
                    .unwrap_or("(yok)");
                cikti.push_str(&format!("  cevrim: {kaynak} -> {hedef}\n"));
            }
        }
        cikti
    }

    /// Bir bileşenin iç kenar kimliklerini döndürür (`kaynak\u{1}hedef`).
    fn bilesen_kenarlari(&self, bilesen: &[String]) -> BTreeSet<(String, String)> {
        let kume: BTreeSet<&str> = bilesen.iter().map(String::as_str).collect();
        self.kenarlar
            .iter()
            .filter(|k| kume.contains(k.kaynak.as_str()) && kume.contains(k.hedef.as_str()))
            .map(|k| (k.kaynak.clone(), k.hedef.clone()))
            .collect()
    }

    /// En çok bağlantı alan notları (derece sırasıyla) döndürür.
    pub fn en_cok_baglantilanan(&self, adet: usize) -> Vec<&Dugum> {
        let mut notlar: Vec<&Dugum> = self
            .dugumler
            .iter()
            .filter(|d| d.tip == DugumTipi::Not)
            .collect();
        notlar.sort_by(|a, b| {
            b.giris
                .cmp(&a.giris)
                .then_with(|| b.cikis.cmp(&a.cikis))
                .then_with(|| a.id.cmp(&b.id))
        });
        notlar.into_iter().take(adet).collect()
    }

    /// Graphviz DOT biçimine çevirir.
    pub fn dot(&self) -> String {
        let mut c = String::new();
        c.push_str("digraph dugumkafa {\n");
        c.push_str("  rankdir=LR;\n");
        c.push_str("  node [shape=box, fontname=\"monospace\"];\n");
        c.push_str("  labelloc=\"t\";\n");
        c.push_str("  label=\"NodeMind bilgi grafigi\";\n");
        for d in &self.dugumler {
            c.push_str(&format!(
                "  \"{}\" [label=\"{} ({}) deg:{}\", type=\"{}\"];\n",
                kacir(&d.id),
                kacir(&d.etiket),
                d.tip.etiket(),
                d.giris + d.cikis,
                d.tip.etiket()
            ));
        }
        for k in &self.kenarlar {
            c.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\", style=\"{}\"];\n",
                kacir(&k.kaynak),
                kacir(&k.hedef),
                k.tip.etiket(),
                if k.tip == KenarTipi::Baglanti {
                    "solid"
                } else {
                    "dotted"
                }
            ));
        }
        c.push_str("}\n");
        c
    }

    /// Bağımsız HTML/SVG sayfasına çevirir (JavaScript yok, dış kaynak yok).
    ///
    /// Düğümler dairesel bir düzende, kenarlar çizgi olarak yerleştirilir.
    /// Yazı tipi ölçüsü ve yarıçap sabittir; bu bir bilgi grafiği **özeti**
    /// görünümüdür, ölçeklendirilebilir bir grafik düzenleyici değildir.
    pub fn html_svg(&self) -> String {
        let n = self.dugum_sayisi();
        let genislik = 1000.0f64;
        let yukseklik = 640.0f64;
        let cx = genislik / 2.0;
        let cy = yukseklik / 2.0;
        let yaricap = 240.0f64.min(if n > 1 {
            genislik.min(yukseklik) / 2.0 - 40.0
        } else {
            0.0
        });
        let konum: BTreeMap<&str, (f64, f64)> = self
            .dugumler
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let aci = 2.0 * std::f64::consts::PI * (i as f64) / (n.max(1) as f64);
                (
                    d.id.as_str(),
                    (cx + yaricap * aci.cos(), cy + yaricap * aci.sin()),
                )
            })
            .collect();

        let mut h = String::new();
        h.push_str("<!DOCTYPE html>\n<html lang=\"tr\">\n<head>\n<meta charset=\"utf-8\">\n");
        h.push_str("<title>NodeMind bilgi grafigi</title>\n");
        h.push_str("<style>body{font-family:monospace;background:#fbfbfd;color:#1d1d20}");
        h.push_str("svg{border:1px solid #ccc;background:#fff}");
        h.push_str(".baglanti{stroke:#2b6cb0;stroke-width:1.4}");
        h.push_str(".yapi{stroke:#b0b4bb;stroke-width:0.8;stroke-dasharray:3 3}");
        h.push_str(".b{fill:#2b6cb0;stroke:#1a4e8a}");
        h.push_str(".t{font-size:9px;fill:#123}</style>\n</head>\n<body>\n");
        h.push_str(&format!(
            "<h1>NodeMind bilgi grafigi</h1><p>{} dugum, {} kenar, {} bilesen, {} cevrim kenar</p>\n",
            n,
            self.kenar_sayisi(),
            self.bilesenler().len(),
            self.cevrim_kenarlari().len()
        ));
        h.push_str(&format!(
            "<svg width=\"{genislik}\" height=\"{yukseklik}\" viewBox=\"0 0 {genislik} {yukseklik}\">\n"
        ));
        for k in &self.kenarlar {
            let (Some(a), Some(b)) = (konum.get(k.kaynak.as_str()), konum.get(k.hedef.as_str()))
            else {
                continue;
            };
            let sinif = if k.tip == KenarTipi::Baglanti {
                "baglanti"
            } else {
                "yapi"
            };
            h.push_str(&format!(
                "<line class=\"{sinif}\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>\n",
                a.0, a.1, b.0, b.1
            ));
        }
        for d in &self.dugumler {
            let (x, y) = konum[d.id.as_str()];
            h.push_str(&format!(
                "<circle class=\"b\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"4\"><title>{}</title></circle>\n",
                kacir_html(&d.etiket)
            ));
            h.push_str(&format!(
                "<text class=\"t\" x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
                x + 6.0,
                y + 3.0,
                kacir_html(&kisa(&d.etiket, 18))
            ));
        }
        h.push_str("</svg>\n</body>\n</html>\n");
        h
    }

    /// Makine-okunur JSON çıktısına çevirir.
    pub fn json(&self) -> Result<String, Hata> {
        serde_json::to_string_pretty(self).map_err(|e| Hata::CiktiHatasi {
            yol: std::path::PathBuf::from("<bellek>"),
            kaynak: std::io::Error::other(e),
        })
    }
}

/// Bir blok ağacının tüm düğümlerini (seçime uyanları) haritaya ekler.
fn dugum_ekle(blok: &Blok, secim: DugumSecimi, dugumler: &mut BTreeMap<String, Dugum>) {
    if secim.kapsar(blok.tip) {
        dugumler.insert(
            blok.kimlik.clone(),
            Dugum {
                id: blok.kimlik.clone(),
                etiket: kisa(&blok.metin, 40),
                tip: DugumTipi::bloktan(blok.tip),
                cikis: 0,
                giris: 0,
            },
        );
    }
    for cocuk in &blok.cocuklar {
        dugum_ekle(cocuk, secim, dugumler);
    }
}

/// Bir blok ağacının ebeveyn → çocuk kenarlarını üretir.
///
/// Not → kök blok kenarı `Graf::olustur` içinde eklenir; burada yalnızca ağacın
/// kendi iç yapısı işlenir. **Her iki uç da seçime uyuyorsa** kenar eklenir;
/// aksi hâlde graf, hiçbir düğümü olmayan bir kenara (çıkmış düğüm) sahip olurdu.
fn agac_kenarlar(
    blok: &Blok,
    secim: DugumSecimi,
    kenarlar: &mut BTreeSet<(String, String, KenarTipi)>,
) {
    for cocuk in &blok.cocuklar {
        if secim.kapsar(blok.tip) && secim.kapsar(cocuk.tip) {
            kenarlar.insert((blok.kimlik.clone(), cocuk.kimlik.clone(), KenarTipi::Ice));
        }
        agac_kenarlar(cocuk, secim, kenarlar);
    }
}

/// Bir bileşeni ASCII ağacı olarak çizer.
fn agac_ciz(
    cikti: &mut String,
    bilesen: &[String],
    kenarlar: &BTreeSet<(String, String)>,
    harita: &BTreeMap<&str, &Dugum>,
) {
    let mut komsu: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (a, b) in kenarlar {
        komsu.entry(a.as_str()).or_default().push(b.as_str());
        komsu.entry(b.as_str()).or_default().push(a.as_str());
    }
    for v in komsu.values_mut() {
        v.sort_unstable();
        v.dedup();
    }
    cizgi_yaz(cikti, &bilesen[0], "", true, true, harita);
    let mut ziyaret: BTreeSet<String> = BTreeSet::new();
    ziyaret.insert(bilesen[0].clone());
    cocuk_ciz(cikti, &bilesen[0], "", &komsu, &mut ziyaret, harita);
}

/// Bir düğümün **çocuklarını** ağaç dalları olarak çizer.
///
/// `cizgi_yaz` düğümün kendi satırını basar; bu fonksiyon yalnızca çocukları
/// çağırır. Kök satır `agac_ciz` içinde bir kez basılır — kökü iki kez basmak,
/// grafiğin kendisi doğru olsa bile çizimi yanlış gösterirdi.
fn cocuk_ciz(
    cikti: &mut String,
    kimlik: &str,
    oncul: &str,
    komsu: &BTreeMap<&str, Vec<&str>>,
    ziyaret: &mut BTreeSet<String>,
    harita: &BTreeMap<&str, &Dugum>,
) {
    let kume: BTreeSet<&str> = komsu
        .get(kimlik)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .copied()
        .filter(|c| ziyaret.insert((*c).to_string()))
        .collect();
    let adet = kume.len();
    let dal_oncul = format!("{oncul}|   ");
    for (i, cocuk) in kume.iter().enumerate() {
        let son = i + 1 == adet;
        cizgi_yaz(cikti, cocuk, &dal_oncul, son, false, harita);
        let alt = if son { "    " } else { "|   " };
        cocuk_ciz(
            cikti,
            cocuk,
            &format!("{dal_oncul}{alt}"),
            komsu,
            ziyaret,
            harita,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn cizgi_yaz(
    cikti: &mut String,
    kimlik: &str,
    oncul: &str,
    son: bool,
    kok: bool,
    harita: &BTreeMap<&str, &Dugum>,
) {
    let (etiket, tip) = harita
        .get(kimlik)
        .map(|x| (x.etiket.as_str(), x.tip.etiket()))
        .unwrap_or(("", "?"));
    if kok {
        cikti.push_str(&format!("  [{tip}] {etiket}\n"));
    } else {
        let dal = if son { "`--" } else { "+--" };
        cikti.push_str(&format!("  {oncul}{dal} [{tip}] {etiket}\n"));
    }
}

/// Bir çözümlenmiş bağlantının hedef düğüm kimliğini verir.
fn hedef_dugum(c: &Cozum, duzce: &Duzce) -> Option<String> {
    match c.tip {
        BaglantiTipi::BlokKimligi => duzce.blok_ara(&c.hedef).map(|_| c.hedef.clone()),
        BaglantiTipi::NotAdi => {
            let adaylar = duzce.not_adaylari(&c.hedef);
            if adaylar.len() == 1 {
                Some(adaylar[0].clone())
            } else {
                None
            }
        }
    }
}

/// DOT ve HTML çıktısında tırnak kaçışı.
fn kacir(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// HTML çıktısında karakter kaçışı.
fn kacir_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::test_yardimcisi::GeciciDizin;
    use std::path::Path;

    fn yaz(d: &Path, goreli: &str, icerik: &str) {
        let yol = d.join(goreli);
        if let Some(ust) = yol.parent() {
            std::fs::create_dir_all(ust).unwrap();
        }
        std::fs::write(yol, icerik).unwrap();
    }

    /// Elle kurulmuş graf: testler ayrıştırıcıdan bağımsız olsun.
    fn el_grafi(dugumler: &[(&str, DugumTipi)], kenarlar: &[(&str, &str, KenarTipi)]) -> Graf {
        Graf {
            dugumler: dugumler
                .iter()
                .map(|(id, tip)| Dugum {
                    id: (*id).to_string(),
                    etiket: (*id).to_string(),
                    tip: *tip,
                    cikis: 0,
                    giris: 0,
                })
                .collect(),
            kenarlar: kenarlar
                .iter()
                .map(|(a, b, t)| Kenar {
                    kaynak: (*a).to_string(),
                    hedef: (*b).to_string(),
                    tip: *t,
                })
                .collect(),
        }
    }

    // ---- bilesenler ----

    #[test]
    fn dugumsuz_graf_bileseni_yoktur() {
        let g = el_grafi(&[], &[]);
        assert!(g.bilesenler().is_empty());
        assert!(g.cevrim_kenarlari().is_empty());
    }

    #[test]
    fn tek_dugum_tek_bilesen_verir() {
        let g = el_grafi(&[("a", DugumTipi::Not)], &[]);
        let b = g.bilesenler();
        assert_eq!(b.len(), 1);
        assert_eq!(b[0], vec!["a".to_string()]);
    }

    #[test]
    fn iki_bagimsiz_dugum_iki_bilesen_verir() {
        let g = el_grafi(&[("a", DugumTipi::Not), ("b", DugumTipi::Not)], &[]);
        assert_eq!(g.bilesenler().len(), 2);
    }

    #[test]
    fn zincir_tek_bilesen_verir() {
        let g = el_grafi(
            &[
                ("a", DugumTipi::Not),
                ("b", DugumTipi::Not),
                ("c", DugumTipi::Not),
            ],
            &[
                ("a", "b", KenarTipi::Baglanti),
                ("b", "c", KenarTipi::Baglanti),
            ],
        );
        assert_eq!(g.bilesenler().len(), 1);
    }

    // ---- donguler ----

    #[test]
    fn iki_dugumlu_dongu_kirilir() {
        let g = el_grafi(
            &[("a", DugumTipi::Not), ("b", DugumTipi::Not)],
            &[
                ("a", "b", KenarTipi::Baglanti),
                ("b", "a", KenarTipi::Baglanti),
            ],
        );
        let c = g.cevrim_kenarlari();
        assert_eq!(c.len(), 1);
        assert_eq!(g.bilesenler().len(), 1);
    }

    #[test]
    fn uc_dugumlu_dongu_kirilir() {
        let g = el_grafi(
            &[
                ("a", DugumTipi::Not),
                ("b", DugumTipi::Not),
                ("c", DugumTipi::Not),
            ],
            &[
                ("a", "b", KenarTipi::Baglanti),
                ("b", "c", KenarTipi::Baglanti),
                ("c", "a", KenarTipi::Baglanti),
            ],
        );
        assert_eq!(g.cevrim_kenarlari().len(), 2);
    }

    #[test]
    fn kendi_kendine_kenar_dongu_sayilir() {
        let g = el_grafi(&[("a", DugumTipi::Not)], &[("a", "a", KenarTipi::Baglanti)]);
        assert_eq!(g.cevrim_kenarlari().len(), 1);
        assert_eq!(g.bilesenler().len(), 1);
    }

    #[test]
    fn dongusuz_agacta_cevrim_yoktur() {
        let g = el_grafi(
            &[
                ("a", DugumTipi::Not),
                ("b", DugumTipi::Not),
                ("c", DugumTipi::Not),
            ],
            &[("a", "b", KenarTipi::Yapi), ("a", "c", KenarTipi::Yapi)],
        );
        assert!(g.cevrim_kenarlari().is_empty());
    }

    #[test]
    fn kopuk_bilesenler_ayri_ayri_cizilir() {
        let g = el_grafi(
            &[
                ("a", DugumTipi::Not),
                ("b", DugumTipi::Not),
                ("c", DugumTipi::Not),
                ("d", DugumTipi::Not),
            ],
            &[("a", "b", KenarTipi::Baglanti)],
        );
        let b = g.bilesenler();
        assert_eq!(b.len(), 3);
        assert!(b.iter().any(|x| x.len() == 2));
        assert!(b.iter().any(|x| x.len() == 1));
    }

    // ---- ascii ----

    #[test]
    fn ascii_bos_graf_mesaji_yazar() {
        let s = el_grafi(&[], &[]).ascii();
        assert!(s.contains("0 dugum"));
        assert!(s.contains("graf bos"));
    }

    #[test]
    fn ascii_tek_dugum_tek_bilesen_yazar() {
        let s = el_grafi(&[("tek", DugumTipi::Not)], &[]).ascii();
        assert!(s.contains("bilesen 1 - 1 dugum, 0 kenar"));
        assert!(s.contains("[not] tek"));
    }

    #[test]
    fn ascii_kopuk_bilesenleri_numaralandirir() {
        let s = el_grafi(&[("a", DugumTipi::Not), ("b", DugumTipi::Not)], &[]).ascii();
        assert!(s.contains("bilesen 1"));
        assert!(s.contains("bilesen 2"));
        assert!(s.contains("2 bilesen"));
    }

    #[test]
    fn ascii_donguyu_cevrim_satirinda_gosterir() {
        let s = el_grafi(
            &[("a", DugumTipi::Not), ("b", DugumTipi::Not)],
            &[
                ("a", "b", KenarTipi::Baglanti),
                ("b", "a", KenarTipi::Baglanti),
            ],
        )
        .ascii();
        // BFS ağacı a->b'yi alır; geri kenar b->a döngü olarak listelenir.
        assert!(s.contains("cevrim: b -> a"), "{s}");
        assert!(s.contains("1 cevrim"));
    }

    #[test]
    fn ascii_dal_isaretleri_kullanir() {
        let s = el_grafi(
            &[
                ("kok", DugumTipi::Not),
                ("c1", DugumTipi::Madde),
                ("c2", DugumTipi::Madde),
            ],
            &[
                ("kok", "c1", KenarTipi::Yapi),
                ("kok", "c2", KenarTipi::Yapi),
            ],
        )
        .ascii();
        assert!(s.contains("`--"), "{s}");
        assert!(s.contains("+--"), "ilk cocuk `+--` ile baslamali: {s}");
    }

    #[test]
    fn ascii_kok_dugumu_yalnizca_bir_kez_basar() {
        let s = el_grafi(
            &[("kok", DugumTipi::Not), ("c1", DugumTipi::Madde)],
            &[("kok", "c1", KenarTipi::Yapi)],
        )
        .ascii();
        assert_eq!(
            s.matches("[not] kok").count(),
            1,
            "kok iki kez basildi:\n{s}"
        );
        assert_eq!(s.matches("[madde] c1").count(), 1, "{s}");
    }

    #[test]
    fn ascii_koku_not_dugumleri_arasindan_secer() {
        // Not dugumu kimlik sirasi daha buyuk olsa bile kok olarak secilir.
        let g = Graf {
            dugumler: vec![
                Dugum {
                    id: "aaa-baslik".to_string(),
                    etiket: "Baslik".to_string(),
                    tip: DugumTipi::Baslik,
                    cikis: 0,
                    giris: 0,
                },
                Dugum {
                    id: "zzz-not".to_string(),
                    etiket: "Not".to_string(),
                    tip: DugumTipi::Not,
                    cikis: 0,
                    giris: 0,
                },
            ],
            kenarlar: vec![Kenar {
                kaynak: "zzz-not".to_string(),
                hedef: "aaa-baslik".to_string(),
                tip: KenarTipi::Yapi,
            }],
        };
        let s = g.ascii();
        assert!(s.contains("\n  [not] Not\n"), "{s}");
    }

    #[test]
    fn ascii_etiketi_kiraltir() {
        let uzun = "cok uzun bir baslik metni burada ve devam ediyor ve daha da uzun";
        assert!(kisa(uzun, 20).chars().count() <= 20);
        assert!(kisa("kisa", 20).ends_with("kisa"));
    }

    // ---- disa aktarim ----

    #[test]
    fn dot_dosyasi_gecerli_gorunur() {
        let g = el_grafi(
            &[("a", DugumTipi::Not), ("b", DugumTipi::Baslik)],
            &[("a", "b", KenarTipi::Baglanti)],
        );
        let d = g.dot();
        assert!(d.starts_with("digraph dugumkafa {"));
        assert!(d.trim_end().ends_with('}'));
        assert!(d.contains("\"a\" -> \"b\""));
        assert!(d.contains("type=\"not\""));
    }

    #[test]
    fn dot_yapı_kenarlarini_kesikli_cizer() {
        let g = el_grafi(
            &[("a", DugumTipi::Not), ("b", DugumTipi::Not)],
            &[("a", "b", KenarTipi::Yapi)],
        );
        assert!(g.dot().contains("style=\"dotted\""));
    }

    #[test]
    fn dot_tirnak_kacisini_yapar() {
        let g = el_grafi(&[("a\"b", DugumTipi::Not)], &[]);
        assert!(g.dot().contains("a\\\"b"));
    }

    #[test]
    fn html_sayfasi_bagimsiz_ve_dogru_uretir() {
        let g = el_grafi(
            &[("a", DugumTipi::Not), ("b", DugumTipi::Not)],
            &[("a", "b", KenarTipi::Baglanti)],
        );
        let h = g.html_svg();
        assert!(h.starts_with("<!DOCTYPE html>"));
        assert!(h.contains("<svg"));
        assert!(h.contains("</svg>"));
        assert!(h.contains("<circle"));
        assert!(h.contains("<line"));
        assert!(!h.contains("<script"), "JavaScript icermemeli");
        assert!(
            !h.contains("http://") && !h.contains("https://"),
            "dizinde kaynak olmamali"
        );
    }

    #[test]
    fn html_bos_grafta_da_gecerlidir() {
        let h = el_grafi(&[], &[]).html_svg();
        assert!(h.contains("0 dugum"));
        assert!(h.contains("</svg>"));
    }

    #[test]
    fn html_etiketleri_kacirir() {
        let g = Graf {
            dugumler: vec![Dugum {
                id: "n1".to_string(),
                etiket: "<script>&".to_string(),
                tip: DugumTipi::Not,
                cikis: 0,
                giris: 0,
            }],
            kenarlar: vec![],
        };
        let h = g.html_svg();
        assert!(!h.contains("<script>"));
        assert!(h.contains("&lt;script&gt;"));
    }

    #[test]
    fn json_gidis_donusu_kimliktir() {
        let g = el_grafi(&[("a", DugumTipi::Not)], &[]);
        let metin = g.json().unwrap();
        let b: Graf = serde_json::from_str(&metin).unwrap();
        assert_eq!(g, b);
        assert!(metin.contains("\"tip\": \"not\""));
    }

    // ---- kasadan uretim ----

    #[test]
    fn kasadan_uretilen_graf_yapi_kenari_icerir() {
        let d = GeciciDizin::yeni("nodemind-graf-uret").unwrap();
        yaz(d.yol(), "a.md", "# A\n\n- bir\n- iki\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let g = Graf::olustur(&k, DugumSecimi::Hepsi);
        assert_eq!(g.dugum_sayisi(), 4);
        assert_eq!(g.kenar_sayisi(), 3);
    }

    #[test]
    fn dugum_secimi_notlari_temizler() {
        let d = GeciciDizin::yeni("nodemind-graf-secim").unwrap();
        yaz(d.yol(), "a.md", "# A\n\n- bir\n");
        let k = Kasa::ac(d.yol()).unwrap();
        assert_eq!(Graf::olustur(&k, DugumSecimi::Not).dugum_sayisi(), 1);
        assert_eq!(Graf::olustur(&k, DugumSecimi::NotBaslik).dugum_sayisi(), 2);
        assert_eq!(Graf::olustur(&k, DugumSecimi::Hepsi).dugum_sayisi(), 3);
    }

    #[test]
    fn baglanti_kenari_ve_derece_hesaplanir() {
        let d = GeciciDizin::yeni("nodemind-graf-derece").unwrap();
        yaz(d.yol(), "hedef.md", "# Hedef\n");
        yaz(d.yol(), "kaynak1.md", "# K1\n\n[[hedef]]\n");
        yaz(d.yol(), "kaynak2.md", "# K2\n\n[[hedef]]\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let g = Graf::olustur(&k, DugumSecimi::Hepsi);
        let hedef = k.not_ara("hedef")[0].id.clone();
        let d = g.dugumler.iter().find(|x| x.id == hedef).unwrap();
        assert_eq!(d.giris, 2);
        let en = g.en_cok_baglantilanan(1);
        assert_eq!(en.len(), 1);
        assert_eq!(en[0].id, hedef);
    }

    #[test]
    fn kirik_baglanti_graf_kenari_uretmez() {
        let d = GeciciDizin::yeni("nodemind-graf-kirik").unwrap();
        yaz(d.yol(), "a.md", "# A\n\n[[olmayan]]\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let g = Graf::olustur(&k, DugumSecimi::Hepsi);
        assert!(g.kenarlar.iter().all(|e| e.tip != KenarTipi::Baglanti));
    }

    #[test]
    fn en_cok_baglantilanan_listesi_kararli() {
        let g = Graf {
            dugumler: vec![
                Dugum {
                    id: "a".to_string(),
                    etiket: "a".to_string(),
                    tip: DugumTipi::Not,
                    cikis: 0,
                    giris: 0,
                },
                Dugum {
                    id: "b".to_string(),
                    etiket: "b".to_string(),
                    tip: DugumTipi::Not,
                    cikis: 5,
                    giris: 0,
                },
                Dugum {
                    id: "c".to_string(),
                    etiket: "c".to_string(),
                    tip: DugumTipi::Not,
                    cikis: 1,
                    giris: 9,
                },
            ],
            kenarlar: vec![],
        };
        let l = g.en_cok_baglantilanan(2);
        assert_eq!(l[0].id, "c");
        assert_eq!(l[1].id, "b");
    }

    #[test]
    fn dugum_tipi_etiketleri_ayrilir() {
        assert_eq!(DugumTipi::Not.etiket(), "not");
        assert_eq!(DugumTipi::Madde.etiket(), "madde");
        assert_eq!(KenarTipi::Baglanti.etiket(), "baglanti");
        assert_eq!(DugumTipi::bloktan(BlokTipi::Kod), DugumTipi::Kod);
    }
}
