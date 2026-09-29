//! DüğümKafa (NodeMind) komut satırı arayüzü.
//!
//! Bu dosya yalnızca komut satırı ayrıştırma, çıktı biçimlendirme ve hata
//! yazımı yapar; Markdown ayrıştırma, referans çözümleme, indeks ve grafik
//! mantığı `nodemind` kütüphanesindedir.
//!
//! Alt komutlar: `new`, `index`, `search`, `backlinks`, `graph`, `today`,
//! `check`.
//!
//! Çıkış kodu: `0` başarı, `1` hata, `2` kullanım hataları `check` sırasında
//! kırık/belirsiz bağlantı bulundu (kabul kriteri), `3` kullanım hatası
//! (clap'in kendi kodu).

#![forbid(unsafe_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use nodemind::cozum::ters_baglantilar;
use nodemind::graf::{DugumSecimi, Graf};
use nodemind::gunluk::{gunluk_sablonu, hafta_gunleri, Tarih};
use nodemind::hata::Hata;
use nodemind::indeks::TersIndeks;
use nodemind::kasa::{ad_dogrula, kasa_ici_mi, Kasa};
use nodemind::markdown::BlokTipi;
use nodemind::sema::{IndeksBelge, SEMA_SURUMU};

/// Varsayılan kasa adı; yürütülebilirin yanında veya çalışma dizininde aranır.
const VARSAYILAN_KASA: &str = "notlar";

/// DüğümKafa — düz Markdown üzerinde çalışan terminal bilgi yöneticisi.
#[derive(Debug, Parser)]
#[command(
    name = "nodemind",
    version,
    about = "Duz Markdown notlarinda blok referansi, tam metin arama ve bilgi grafigi uretir.",
    long_about = "NodeMind, not klasorunu (.md dosyalari) okur; hicbir not dosyasina \
                  YAZMAZ. Blok kimlikleri icerikten turetilir, indeks cache/index.json \
                  altinda tutulur ve silinebilir."
)]
struct Cli {
    /// Not klasörü (kasa). Verilmezse sırayla: ./notlar, yürütülebilir yanı/notlar,
    /// çalışma dizini.
    #[arg(long, short = 'v', value_name = "YOL")]
    kasa: Option<PathBuf>,

    #[command(subcommand)]
    komut: Komut,
}

/// Arama sonucu süzgecinin karşılığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum TurArg {
    /// Başlıklar.
    Baslik,
    /// Liste maddeleri.
    Madde,
    /// Alıntılar.
    Alinti,
    /// Kod blokları.
    Kod,
    /// Paragraflar.
    Paragraf,
}

impl TurArg {
    fn blogu(self) -> BlokTipi {
        match self {
            TurArg::Baslik => BlokTipi::Baslik,
            TurArg::Madde => BlokTipi::Liste,
            TurArg::Alinti => BlokTipi::Alinti,
            TurArg::Kod => BlokTipi::Kod,
            TurArg::Paragraf => BlokTipi::Paragraf,
        }
    }
}

/// Düğüm seçiminin karşılığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum DugumArg {
    /// Yalnızca not düğümleri.
    Not,
    /// Notlar ve başlıklar.
    NotBaslik,
    /// Notlar ve tüm bloklar.
    Hepsi,
}

impl DugumArg {
    fn secim(self) -> DugumSecimi {
        match self {
            DugumArg::Not => DugumSecimi::Not,
            DugumArg::NotBaslik => DugumSecimi::NotBaslik,
            DugumArg::Hepsi => DugumSecimi::Hepsi,
        }
    }
}

/// Grafik çıktı biçiminin karşılığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum BicimArg {
    /// Terminal ASCII çizimi.
    Ascii,
    /// Makine-okunur JSON.
    Json,
    /// Graphviz DOT.
    Dot,
    /// Bağımsız HTML/SVG sayfası.
    Html,
}

impl BicimArg {
    fn ad(self) -> &'static str {
        match self {
            BicimArg::Ascii => "ascii",
            BicimArg::Json => "json",
            BicimArg::Dot => "dot",
            BicimArg::Html => "html",
        }
    }
}

/// Günlük görünümünün karşılığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum GunlukArg {
    /// Belirtilen günün günlük notu.
    Gun,
    /// Belirtilen ISO haftasının yedi günü.
    Hafta,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match calistir(cli) {
        Ok(kod) => kod,
        Err(h) => {
            eprintln!("hata: {h}");
            ExitCode::from(1)
        }
    }
}

/// Kasa yolunu çözer (rapor b09, şema 4 yol çözümlemesi).
fn kasa_coz(istenen: Option<&Path>) -> PathBuf {
    if let Some(p) = istenen {
        return p.to_path_buf();
    }
    let mut adaylar: Vec<PathBuf> = Vec::new();
    if let Ok(c) = std::env::current_dir() {
        adaylar.push(c.join(VARSAYILAN_KASA));
    }
    if let Ok(e) = std::env::current_exe() {
        if let Some(p) = e.parent() {
            adaylar.push(p.join(VARSAYILAN_KASA));
        }
    }
    adaylar
        .into_iter()
        .find(|a| a.is_dir())
        .unwrap_or_else(|| PathBuf::from(VARSAYILAN_KASA))
}

fn kasa_ac(istenen: Option<&Path>) -> Result<(Kasa, PathBuf), Hata> {
    let yol = kasa_coz(istenen);
    let kasa = Kasa::ac(&yol)?;
    Ok((kasa, yol))
}

fn calistir(cli: Cli) -> Result<ExitCode, Hata> {
    match &cli.komut {
        Komut::New { ad, baslik, icerik } => {
            komut_new(&cli, ad, baslik.as_deref(), icerik.as_deref())
        }
        Komut::Index { sessiz } => komut_index(&cli, *sessiz),
        Komut::Search {
            sorgu,
            tur,
            not,
            limit,
        } => komut_search(&cli, sorgu, *tur, not.as_deref(), *limit),
        Komut::Backlinks { hedef, tur } => komut_backlinks(&cli, hedef, *tur),
        Komut::Graph {
            bicim,
            dugum,
            cikti,
            en_cok,
        } => komut_graph(&cli, *bicim, *dugum, cikti.as_deref(), *en_cok),
        Komut::Today {
            bugun,
            tarih,
            gorunum,
            yaz,
        } => komut_today(&cli, *bugun, tarih.as_deref(), *gorunum, *yaz),
        Komut::Check { json } => komut_check(&cli, *json),
    }
}

/// Alt komutlar.
#[derive(Debug, Subcommand)]
enum Komut {
    /// Yeni bir not dosyası oluşturur. Var olan bir not **asla** üzerine yazılmaz.
    New {
        /// Not adı (uzantısız). Yol ayracı ve `..` reddedilir.
        ad: String,
        /// Notun `#` başlığı (verilmezse ad kullanılır).
        #[arg(long, value_name = "METIN")]
        baslik: Option<String>,
        /// Gövde içeriği; verilmezse boş bir şablon oluşturulur.
        #[arg(long, value_name = "METIN")]
        icerik: Option<String>,
    },
    /// Not klasörünü tarar ve `cache/index.json` indeksini (yeniden) kurar.
    Index {
        /// Yalnızca sonucu yaz, ayrıntı verme.
        #[arg(long, short = 'q')]
        sessiz: bool,
    },
    /// Tam metin araması yapar (ters indeks; Türkçe normalizasyonlu).
    Search {
        /// Arama sorgusu. Birden çok sözcük **ve** olarak birleştirilir.
        #[arg(required = true, num_args = 1.., value_name = "SORGU")]
        sorgu: Vec<String>,
        /// Yalnızca bu blok türünde ara.
        #[arg(long, short = 't', value_name = "TUR")]
        tur: Option<TurArg>,
        /// Yalnızca bu notta ara (not adı veya göreli yol).
        #[arg(long, short = 'n', value_name = "NOT")]
        not: Option<String>,
        /// En fazla bu kadar sonuç göster (0 = sınırsız).
        #[arg(long, short = 'l', default_value_t = 20, value_name = "N")]
        limit: usize,
    },
    /// Verilen nota veya bloğa **hangi blokların bağlandığını** listeler.
    Backlinks {
        /// Not adı, göreli yol veya blok kimliği.
        hedef: String,
        /// Yalnızca bu lehçedeki bağlantıları göster.
        #[arg(long, value_name = "TUR", value_enum)]
        tur: Option<BaglantiTurArg>,
    },
    /// Bilgi grafiğini çizer veya dışa aktarır.
    Graph {
        /// Çıktı biçimi.
        #[arg(long, short = 'b', value_enum, default_value = "ascii")]
        bicim: BicimArg,
        /// Grafa alınacak düğüm türleri.
        #[arg(long, short = 'd', value_enum, default_value = "hepsi")]
        dugum: DugumArg,
        /// Çıktıyı bu dosyaya yaz (kasa dışına yazılamaz).
        #[arg(long, short = 'o', value_name = "DOSYA")]
        cikti: Option<PathBuf>,
        /// "En çok bağlantı alanlar" listesinde kaç not gösterilecek.
        #[arg(long, value_name = "N", default_value_t = 5)]
        en_cok: usize,
    },
    /// Günlük/haftalık akış: bugünün (veya seçilen tarihin) günlüğünü gösterir.
    Today {
        /// Bugünün günlüğünü gösterir (varsayılan davranış; açıkça yazılabilir).
        #[arg(long, visible_alias = "bugün")]
        bugun: bool,
        /// `bugun` (varsayılan) veya `YYYY-AA-GG`. `--bugun` ile birlikte verilemez.
        #[arg(long, value_name = "TARIH")]
        tarih: Option<String>,
        /// Gün yerine ISO haftasının yedi gününü listeler.
        #[arg(long, value_enum, default_value = "gun")]
        gorunum: GunlukArg,
        /// Günlük notu yoksa şablonla **oluşturur** (var olan dosyaya yazmaz).
        #[arg(long, short = 'y')]
        yaz: bool,
    },
    /// Kırık referansları, yinelenen adları ve şema durumunu denetler.
    Check {
        /// Makine-okunur JSON raporu yazar.
        #[arg(long)]
        json: bool,
    },
}

/// `backlinks --tur` seçeneğinin karşılığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum BaglantiTurArg {
    /// Yalnızca `[[not adı]]`.
    Not,
    /// Yalnızca `((blok kimligi))`.
    Blok,
}

// ---------------------------------------------------------------- komutlar

fn komut_new(
    cli: &Cli,
    ad: &str,
    baslik: Option<&str>,
    icerik: Option<&str>,
) -> Result<ExitCode, Hata> {
    ad_dogrula(ad)?;
    let (kasa, kok) = kasa_ac(cli.kasa.as_deref())?;
    let bas = baslik.unwrap_or(ad);
    let govde = match icerik {
        Some(i) => format!("# {bas}\n\n{i}\n"),
        None => format!("# {bas}\n\n- \n"),
    };
    let yol = kasa.yeni_not(ad, &govde)?;
    println!("olusturuldu: {}", yol.display());
    println!("kasa: {}", kok.display());
    Ok(ExitCode::SUCCESS)
}

fn komut_index(cli: &Cli, sessiz: bool) -> Result<ExitCode, Hata> {
    let (kasa, kok) = kasa_ac(cli.kasa.as_deref())?;
    let olusturma = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let belge = nodemind::indeks_kur(&kasa, olusturma);
    let yol = kasa.indeks_yolu();
    belge.kaydet(&yol)?;
    if !sessiz {
        println!("kasa: {}", kok.display());
        println!("indeks: {}", yol.display());
        println!("not: {}  blok: {}", belge.not_sayisi, belge.blok_sayisi);
        println!(
            "terim: {}  posting: {}",
            belge.terimler.len(),
            terim_toplam(&belge)
        );
        println!("sema surumu: {SEMA_SURUMU}  ikili surumu: {}", belge.surum);
    }
    for u in &kasa.uyarilar {
        eprintln!("uyari: {u}");
    }
    Ok(ExitCode::SUCCESS)
}

fn terim_toplam(belge: &IndeksBelge) -> usize {
    belge
        .terimler
        .values()
        .map(std::collections::BTreeMap::len)
        .sum()
}

fn komut_search(
    cli: &Cli,
    sorgu: &[String],
    tur: Option<TurArg>,
    not: Option<&str>,
    limit: usize,
) -> Result<ExitCode, Hata> {
    let (kasa, _) = kasa_ac(cli.kasa.as_deref())?;
    let belge = indeksi_getir_veya_kur(&kasa);
    let terimler = belgeden_indeks(&belge);
    let bloklar = belgeden_bloklar(&belge);

    // `--not` süzgeci çözülür: ad veya göreli yoldan not kimliğine.
    let not_id = match not {
        None => None,
        Some(secim) => {
            let adaylar = kasa.not_ara(secim);
            match adaylar.first() {
                Some(n) => Some(n.id.clone()),
                None => {
                    eprintln!("uyari: `{secim}` adli not bulunamadi; filtre uygulanmadi");
                    None
                }
            }
        }
    };

    let sonuclar = terimler.sira(
        &sorgu.join(" "),
        &bloklar,
        tur.map(TurArg::blogu),
        not_id.as_deref(),
    );
    if sonuclar.is_empty() {
        println!("eslesme yok: {}", sorgu.join(" "));
        return Ok(ExitCode::SUCCESS);
    }
    let gosterilecek = if limit == 0 {
        sonuclar.len()
    } else {
        sonuclar.len().min(limit)
    };
    println!("{} eslesme ({} gosteriliyor)", sonuclar.len(), gosterilecek);
    for s in sonuclar.iter().take(gosterilecek) {
        // Başlık yolu notun H1 başlığıyla başlar; o zaten dosya adından okunduğu
        // için çıktıda tekrarlanmaz.
        let onek = format!("{} > ", s.baslik);
        let bolum = s
            .baslik_yolu
            .strip_prefix(onek.as_str())
            .unwrap_or(&s.baslik_yolu);
        let yol = if bolum.is_empty() || bolum == s.baslik {
            s.not_yol.clone()
        } else {
            format!("{} > {}", s.not_yol, bolum)
        };
        println!("[{}] {} :: {}", s.tur.etiket(), yol, s.ozet);
    }
    Ok(ExitCode::SUCCESS)
}

fn komut_backlinks(cli: &Cli, hedef: &str, tur: Option<BaglantiTurArg>) -> Result<ExitCode, Hata> {
    let (kasa, _) = kasa_ac(cli.kasa.as_deref())?;
    let harita = ters_baglantilar(&kasa);

    // Önce blok kimliği, sonra not adı/yolu dene.
    let mut anahtarlar: Vec<String> = Vec::new();
    if let Some((not, ozet)) = kasa.blok_ara(hedef) {
        anahtarlar.push(ozet.kimlik.clone());
        let _ = not;
    }
    for n in kasa.not_ara(hedef) {
        if !anahtarlar.contains(&n.id) {
            anahtarlar.push(n.id.clone());
        }
    }
    if anahtarlar.is_empty() {
        eprintln!("uyari: `{hedef}` bulunamadi (not adi, goreli yol veya blok kimligi denendi)");
        return Ok(ExitCode::SUCCESS);
    }

    let toplam: usize = anahtarlar
        .iter()
        .filter_map(|a| harita.get(a))
        .map(Vec::len)
        .sum();
    for a in &anahtarlar {
        let baslik = anahtar_basligi(&kasa, a);
        println!("=== {baslik} ({}) ===", a);
        let secili: Vec<_> = harita
            .get(a)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .filter(|g| match tur {
                None => true,
                Some(BaglantiTurArg::Not) => g.tip == nodemind::referans::BaglantiTipi::NotAdi,
                Some(BaglantiTurArg::Blok) => {
                    g.tip == nodemind::referans::BaglantiTipi::BlokKimligi
                }
            })
            .collect();
        if secili.is_empty() {
            println!("  (bana bagli blok yok)");
        }
        for g in secili {
            println!(
                "  {}  {}  satir {} — {}",
                g.kaynak_not,
                g.tip.aciklama(),
                g.satir,
                g.blok_baslik_yolu
            );
        }
    }
    println!("toplam: {toplam} ters baglanti");
    Ok(ExitCode::SUCCESS)
}

/// Not kimliğini kullanıcının göreceği göreli yola çevirir.
///
/// `check` çıktısı okunabilir olmalıdır: kullanıcı 12 haneli bir karma değil,
/// dosya yolunu görmek ister. Kimlik bulunamazsa (silinmiş not) `?` yazılır.
fn not_yolu(kasa: &Kasa, kimlik: &str) -> String {
    kasa.notlar
        .iter()
        .find(|n| n.id == kimlik)
        .map(|n| n.yol.clone())
        .unwrap_or_else(|| "(bilinmeyen)".to_string())
}

fn anahtar_basligi(kasa: &Kasa, kimlik: &str) -> String {
    if let Some((not, _)) = kasa.blok_ara(kimlik) {
        return format!("{} (blok {})", not.baslik, kimlik);
    }
    kasa.notlar
        .iter()
        .find(|n| n.id == kimlik)
        .map(|n| n.baslik.clone())
        .unwrap_or_else(|| "(bilinmeyen)".to_string())
}

fn komut_graph(
    cli: &Cli,
    bicim: BicimArg,
    dugum: DugumArg,
    cikti: Option<&Path>,
    en_cok: usize,
) -> Result<ExitCode, Hata> {
    let (kasa, _) = kasa_ac(cli.kasa.as_deref())?;
    let g = Graf::olustur(&kasa, dugum.secim());

    let metin = match bicim {
        BicimArg::Ascii => {
            let mut s = g.ascii();
            if en_cok > 0 {
                s.push_str("\nen cok baglantilanan notlar:\n");
                for d in g.en_cok_baglantilanan(en_cok) {
                    s.push_str(&format!("  {:>4}  {}\n", d.giris + d.cikis, d.etiket));
                }
            }
            s
        }
        BicimArg::Json => g.json()?,
        BicimArg::Dot => g.dot(),
        BicimArg::Html => g.html_svg(),
    };

    match cikti {
        Some(ham) => {
            if !kasa_ici_mi(&kasa.kok, ham) {
                return Err(Hata::KasaDisiYol {
                    yol: ham.display().to_string(),
                });
            }
            // Göreli yollar **kasa köküne** göre çözülür; aksi hâlde çıktı
            // çalışma dizinine düşer ve kullanıcı beklediği dosyayı bulamaz.
            let yol = if ham.is_absolute() {
                ham.to_path_buf()
            } else {
                kasa.kok.join(ham)
            };
            if let Some(ust) = yol.parent() {
                if !ust.as_os_str().is_empty() && !ust.exists() {
                    std::fs::create_dir_all(ust)
                        .map_err(|e| nodemind::hata::io_hata("cikti dizini", ust, e))?;
                }
            }
            std::fs::write(&yol, metin).map_err(|e| Hata::CiktiHatasi {
                yol: yol.clone(),
                kaynak: e,
            })?;
            println!("yazildi: {} ({})", yol.display(), bicim.ad());
        }
        None => {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(metin.as_bytes());
            let _ = stdout.flush();
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn komut_today(
    cli: &Cli,
    bugun: bool,
    tarih: Option<&str>,
    gorunum: GunlukArg,
    yaz: bool,
) -> Result<ExitCode, Hata> {
    if bugun && tarih.is_some() {
        return Err(Hata::GecersizSecenek {
            degerler: "--bugun --tarih".to_string(),
            sebep: "ikisi ayni anda verilemez; ya bugun ya tek bir tarih secilir",
        });
    }
    let (kasa, _) = kasa_ac(cli.kasa.as_deref())?;
    let secim = tarih.unwrap_or("bugun");
    let secilen = Tarih::cozumle(secim)?;

    match gorunum {
        GunlukArg::Gun => gunluk_goster(&kasa, &secilen, yaz),
        GunlukArg::Hafta => {
            let gunler = hafta_gunleri(secilen.yil, secilen.iso_hafta())?;
            println!(
                "ISO haftasi {} ({}), {} gun\n",
                secilen.iso_hafta(),
                secilen.yil,
                gunler.len()
            );
            for g in &gunler {
                let varlik = kasa
                    .notlar
                    .iter()
                    .find(|n| n.tarih == Some(*g))
                    .map(|n| n.baslik.clone());
                match varlik {
                    Some(b) => println!("  {}  {}", g.iso_metni(), b),
                    None => println!("  {}  (gunluk yok)", g.iso_metni()),
                }
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn gunluk_goster(kasa: &Kasa, tarih: &Tarih, yaz: bool) -> Result<ExitCode, Hata> {
    let hedef = tarih.dosya_adi();
    let mevcut = kasa.notlar.iter().find(|n| n.yol.ends_with(&hedef));
    match mevcut {
        Some(not) => {
            println!("# {}", not.baslik);
            println!("{} — {}", tarih.iso_metni(), tarih.uzun());
            println!("dosya: {}", not.yol);
            println!("---");
            for ozet in not.duz_bloklar() {
                if ozet.tip == BlokTipi::Baslik {
                    continue;
                }
                println!(
                    "  [{}] {}",
                    ozet.tip.etiket(),
                    ozet.metin.replace('\n', " ")
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        None => {
            println!("# {}", tarih.iso_metni());
            println!("{} — {}", tarih.iso_metni(), tarih.uzun());
            println!("gunluk notu yok: {}", hedef);
            println!("---");
            print!("{}", gunluk_sablonu(tarih));
            if yaz {
                let ad = tarih.iso_metni();
                let yol = kasa.yeni_not(&ad, &gunluk_sablonu(tarih))?;
                println!("olusturuldu: {}", yol.display());
            } else {
                println!();
                println!("olusturmak icin: nodemind today --bugun --yaz");
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn komut_check(cli: &Cli, json: bool) -> Result<ExitCode, Hata> {
    let (kasa, kok) = kasa_ac(cli.kasa.as_deref())?;
    let rapor = nodemind::baglanti_raporu(&kasa);
    let yinelenen = kasa.yinelenen_adlar();

    // İndeks şema durumu: dosya varsa okunur, uyuşmazlık açıkça bildirilir.
    let indeks_yolu = kasa.indeks_yolu();
    let indeks_durumu = if indeks_yolu.exists() {
        match IndeksBelge::yukle(&indeks_yolu) {
            Ok(b) => format!("uyumlu (sema {}, {} not)", b.sema, b.not_sayisi),
            Err(e) => format!("UYUMSUZ: {e}"),
        }
    } else {
        format!("yok (beklenen: {})", indeks_yolu.display())
    };

    if json {
        let veri = serde_json::json!({
            "sema": SEMA_SURUMU,
            "kasa": kok.display().to_string(),
            "not": kasa.notlar.len(),
            "baglanti": {
                "toplam": rapor.toplam(),
                "cozulen": rapor.cozulen.len(),
                "belirsiz": rapor.belirsiz.len(),
                "kirik": rapor.kirik.len(),
                "kendine": rapor.kendine.len(),
            },
            "yinelenen_adlar": yinelenen,
            "indeks": indeks_durumu,
            "kirik_detay": rapor.kirik.iter().map(|c| serde_json::json!({
                "hedef": c.hedef, "not": not_yolu(&kasa, &c.kaynak_not), "blok": c.kaynak_blok,
                "satir": c.satir, "lehce": c.tip.aciklama(),
            })).collect::<Vec<_>>(),
            "belirsiz_detay": rapor.belirsiz.iter().map(|c| serde_json::json!({
                "hedef": c.hedef, "not": not_yolu(&kasa, &c.kaynak_not), "satir": c.satir,
            })).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&veri).unwrap_or_else(|e| format!("{{\"hata\":\"{e}\"}}"))
        );
    } else {
        println!("kasa: {}", kok.display());
        println!("not: {}", kasa.notlar.len());
        println!(
            "baglanti: {} (cozulen {}, belirsiz {}, kirik {})",
            rapor.toplam(),
            rapor.cozulen.len(),
            rapor.belirsiz.len(),
            rapor.kirik.len()
        );
        if !yinelenen.is_empty() {
            println!("\nyinelenen adlar (belirsiz baglanti kaynagi):");
            for (ad, yollar) in &yinelenen {
                println!("  {ad} -> {}", yollar.join(", "));
            }
        }
        if !rapor.kirik.is_empty() {
            println!("\nkirik referanslar:");
            for c in &rapor.kirik {
                println!(
                    "  {}:{}  {}  `{}`",
                    not_yolu(&kasa, &c.kaynak_not),
                    c.satir,
                    c.tip.aciklama(),
                    c.hedef
                );
            }
        }
        if !rapor.belirsiz.is_empty() {
            println!("\nbelirsiz referanslar:");
            for c in &rapor.belirsiz {
                println!(
                    "  {}:{}  `{}`",
                    not_yolu(&kasa, &c.kaynak_not),
                    c.satir,
                    c.hedef
                );
            }
        }
        if !rapor.kendine.is_empty() {
            println!("\nkendine baglanti veren bloklar:");
            for c in &rapor.kendine {
                println!(
                    "  {}:{}  `{}`",
                    not_yolu(&kasa, &c.kaynak_not),
                    c.satir,
                    c.hedef
                );
            }
        }
        println!("\nindeks semasi: {indeks_durumu}");
        for u in &kasa.uyarilar {
            println!("uyari: {u}");
        }
        println!(
            "\nsonuc: {}",
            if rapor.sorunlu_mu() || !yinelenen.is_empty() {
                "SORUNLU"
            } else {
                "TEMIZ"
            }
        );
    }

    if rapor.sorunlu_mu() || !yinelenen.is_empty() {
        Ok(ExitCode::from(2))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

// ------------------------------------------------------------------ indeks

/// `cache/index.json` yoksa ya da bayatıksa kasa taranarak bellekte kurar.
///
/// Bayatlık ölçütü: not dosyasının son değişiklik saniyesi kayıttan farklıysa
/// ya da yeni/eksik not varsa. Bu, "arama sonucu eski" belirsizliğini ortadan
/// kaldırır; kullanıcıya ayrıca bir uyarı yazılır.
fn indeksi_getir_veya_kur(kasa: &Kasa) -> IndeksBelge {
    let yol = kasa.indeks_yolu();
    if !yol.exists() {
        return belgeden_kur(kasa, "uyari: cache/index.json yok, bellekte kuruldu");
    }
    match IndeksBelge::yukle(&yol) {
        Ok(b) if taze_mi(kasa, &b) => b,
        Ok(_) => belgeden_kur(kasa, "uyari: indeks bayat, bellekte yeniden kuruldu"),
        Err(e) => belgeden_kur(kasa, &format!("uyari: {e}")),
    }
}

fn belgeden_kur(kasa: &Kasa, uyari: &str) -> IndeksBelge {
    eprintln!("{uyari}");
    let simdi = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    nodemind::indeks_kur(kasa, simdi)
}

fn taze_mi(kasa: &Kasa, belge: &IndeksBelge) -> bool {
    if belge.not_sayisi != kasa.notlar.len() {
        return false;
    }
    for n in &kasa.notlar {
        let kayit = belge.notlar.iter().find(|x| x.id == n.id);
        match kayit {
            Some(k) if k.degisiklik == n.degisiklik => {}
            _ => return false,
        }
    }
    true
}

fn belgeden_indeks(belge: &IndeksBelge) -> TersIndeks {
    let mut i = TersIndeks::yeni();
    i.terimleri_doldur(&belge.terimler);
    i
}

fn belgeden_bloklar(
    belge: &IndeksBelge,
) -> std::collections::BTreeMap<String, nodemind::sema::BlokKaydi> {
    belge.blok_haritasi()
}
