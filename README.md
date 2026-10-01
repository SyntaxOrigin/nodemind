# DüğümKafa (NodeMind)

Bulut hesabı olmadan, **düz Markdown dosyaları** üzerinde çalışan; blok
referansları, tam metin araması ve bilgi grafiğiyle örülen kişisel bilgi yönetim
aracı.

NodeMind notlarınızı **hiçbir zaman değiştirmez**. Notlarınız `.md` dosyalarıdır;
program kapanırsa bile herhangi bir metin düzenleyicide okunurlar. Tek yazma
noktası `cache/index.json` önbelleğidir ve bu dosya silinebilir, notlardan yeniden
üretilebilir.

> **Veri güvenliği kuralı:** Var olan bir not dosyası **asla** üzerine yazılmaz.
> `nodemind new` ve `nodemind today --yaz` komutları hedef dosya varsa açık bir
> hata döner ve dosyaya dokunmaz.

---

## Özellikler

* **Markdown outliner** — başlık hiyerarşisi, iç içe listeler, alıntılar, kod
  blokları ve paragraflar bir **blok ağacına** çevrilir.
* **Kararlı blok kimliği** — her blok içeriğinden türetilen 12 haneli bir kimlik
  alır; aynı dosya 100 kez açılsa da kimlikler değişmez. Kimlik dosyaya
  yazılmaz, notlar düz metin olarak kalır.
* **İki lehçe bağlantı** — Obsidian tarzı `[[not adı]]` ve Logseq tarzı
  `((blok kimliği))`. `[[ad|etiket]]` de kabul edilir.
* **Belirsizlik ve kırık referans raporu** — çözülemeyen bağlantılar
  sessizce yutulmaz; `[[ad]]` birden çok nota karşılık geliyorsa *belirsiz*,
  hiçbiri yoksa *kırık* olarak ayrı listelerde yazılır.
* **Ters indeksli tam metin arama** — kendi indeksimiz (FTS kütüphanesi yok).
  Türkçe harf katlaması (`İ/I/ı/i` → `i` vb.), basit kök indirgeme ve ünsüz
  yumuşaması, terimler arası **VE** mantığı, alan ağırlıklı sıralama.
* **Ters bağlantılar** — `backlinks` ile "hangi bloklar bana bağlı?" sorusu.
* **Bilgi grafiği** — düğüm/kenar listesi, **terminal ASCII çizimi** (döngüler ve
  kopuk bileşenler ayrı gösterilir) ve JSON / Graphviz DOT / bağımsız HTML-SVG
  dışa aktarımı.
* **Günlük/haftalık akış** — `YYYY-AA-GG.md` tarihli notlar, `today --bugun` ve
  ISO haftası görünümü. Tarih hesabı `std::time` ile kendi kodumuzda yapılır.
* **Çakışma riski belgelenmiş** — not dosyalarına yazılmaz, kilit dosyası yoktur;
  iki araç aynı dosyayı aynı anda düzenlerse son yazan kazanır ve NodeMind bir
  sonraki taramada dosyayı **yeniden okur**. Ayrıntı "Bilinen Sınırlamalar".
* Terminal arayüzü + statik HTML/SVG dışa aktarımı; ağ, pencere ve grafik
  kütüphanesi **yoktur**.

---

## Kurulum

Gereksinim: Rust **1.74** veya üzeri (MSRV) ve bir C bağlantılayıcı
(Linux: `cc`, Windows: MSVC araç zinciri). Başka hiçbir bağımlılık gerekmez.

```console
$ cargo build --release
   Compiling nodemind v0.1.0 (C:\...\projects\21-nodemind)
    Finished `release` profile [optimized] target(s) in 17.97s
```

İkili `target/release/nodemind.exe` (Windows) veya `target/release/nodemind`
(Linux/macOS) olarak üretilir.

```console
$ cargo install --path .
```

Doğrulama:

```console
$ nodemind --version
nodemind 0.1.0
```

---

## Kullanım

Aşağıdaki **her komut** depodaki `ornek/` koleksiyonu üzerinde gerçekten
çalıştırılmıştır; çıktılar kopyalanmıştır. Windows `cmd.exe` yorumlayıcısında
`$` işareti yerine `>` kullanılır.

### Kasa (not klasörü) belirtme

`--kasa` verilmezse sırayla `./notlar`, yürütülebilirin yanındaki `notlar/` ve
çalışma dizini denenir. Örneklerde `--kasa .\ornek` kullanılmıştır.

### 1. `index` — tüm notları tarar ve indeksi kurar

```console
$ nodemind --kasa .\ornek index
kasa: .\ornek
indeks: .\ornek\cache\index.json
not: 10  blok: 78
terim: 410  posting: 873
sema surumu: 1  ikili surumu: 0.1.0
```

`cache/index.json` **türetilmiş veridir**: silinirse bir sonraki `index`
çağrısında yeniden kurulur. Arama komutu bu dosyayı kullanır; dosya bayat
veya şema sürümü uyuşmazsa bellekte yeniden kurar ve stderr'a uyarı yazar.

### 2. `search` — tam metin araması (Türkçe duyarlı)

```console
$ nodemind --kasa .\ornek search "turkce harf"
1 eslesme (1 gosteriliyor)
[madde] Arama.md > Kriterler :: Turkce harf katlamasi yapilir: `I/ı/İ/i` -> `i`, `Ş/s` -> `s`, `Ğ/g` -> `g`, `Ü/u` -> `u`, `Ö/o` -> `o`, `Ç/c` -> `c`.
```

Sorgu ASCII yazılmıştır, metin Türkçe harf içerir: `turkce` sorgusu `Turkce`
başlığını bulur. Sıralama **alan sırası + terim frekansı** ile yapılır; blok
türü süzgeci verilebilir:

```console
$ nodemind --kasa .\ornek search "blok kimligi" --tur baslik
1 eslesme (1 gosteriliyor)
[baslik] Markdown Lehcesi.md > Blok Kimligi :: Blok Kimligi
```

Eşleşme yoksa komut bunu açıkça söyler ve çıkış kodu `0`'dır:

```console
$ nodemind --kasa .\ornek search "olmayan kelime"
eslesme yok: olmayan kelime
```

### 3. `backlinks` — "bana ne bağlı?"

```console
$ nodemind --kasa .\ornek backlinks "Markdown Lehcesi"
=== Markdown Lehcesi (2139c9f8a5cf) ===
  Arama.md  [[not adi]]  satir 18 — Arama > Indeks
  gunluk/2026-09-28.md  [[not adi]]  satir 6 — 2026-09-28 > Bugun
  notlar/calisma-duzeni.md  [[not adi]]  satir 13 — Calisma Duzeni > Gecmis
toplam: 3 ters baglanti
```

Hedef olarak not adı, göreli yol (`gunluk/2026-09-28`) veya blok kimliği
verilebilir. `--tur not` / `--tur blok` yalnızca bir lehçeyi listeler.

### 4. `check` — kırık, belirsiz ve yinelenen ad denetimi

```console
$ nodemind --kasa .\ornek check
kasa: .\ornek
not: 10
baglanti: 14 (cozulen 10, belirsiz 2, kirik 2)

yinelenen adlar (belirsiz baglanti kaynagi):
  2026-09-01 -> arsiv/2026-09-01.md, gunluk/2026-09-01.md

kirik referanslar:
  Bilgi Grafigi.md:2  [[not adi]]  `Olmayan Not`
  gunluk/2026-09-28.md:1  ((blok kimligi))  `yokkimlik`

belirsiz referanslar:
  arsiv/2026-09-01.md:2  `2026-09-01`
  gunluk/2026-09-01.md:1  `2026-09-01`

indeks semasi: uyumlu (sema 1, 10 not)

sonuc: SORUNLU
```

Çıkış kodu: `0` temiz, `1` hata, **`2` kırık veya belirsiz bağlantı var**
(CI'da kullanılabilir). `check --json` makine-okunur rapor verir.

### 5. `graph` — terminal ASCII grafiği

```console
$ nodemind --kasa .\ornek graph --dugum not-baslik
bilgi grafigi: 34 dugum, 24 kenar, 10 bilesen, 0 cevrim kenar

bilesen 1 - 6 dugum, 5 kenar
  [not] Markdown Lehcesi
  |   `-- [baslik] Markdown Lehcesi
  |       |   +-- [baslik] Blok Kimligi
  |       |   +-- [baslik] Blok Turleri
  |       |   +-- [baslik] Alinti
  |       |   `-- [baslik] Desteklenmeyenler

bilesen 2 - 4 dugum, 3 kenar
  [not] Calisma Duzeni
  |   `-- [baslik] Calisma Duzeni
  |       |   +-- [baslik] Kurallar
  |       `-- [baslik] Gecmis

bilesen 3 - 4 dugum, 3 kenar
  [not] Arama
  |   `-- [baslik] Arama
  |       |   +-- [baslik] Kriterler
  |       `-- [baslik] Indeks
...
```

Kopuk bileşenler ayrı ayrı numaralandırılır; döngü yokken `cevrim kenar`
sıfırdır. Döngü **varsa** ağaç kenarı olmayan kenar bileşen altında
`cevrim: a -> b` olarak listelenir — sessizce kaybolmaz.

Varsayılan olarak en çok bağlantı alan notlar da listelenir:

```console
$ nodemind --kasa .\ornek graph --dugum not-baslik --en-cok 3

en cok baglantilanan notlar:
     4  Arama
     4  Bilgi Grafigi
     3  Markdown Lehcesi
```

Dereçe (giriş + çıkış) **not** üzerinden hesaplanır ve `--dugum` filtresinden
bağımsızdır: bağlantı kaynağı çizimde olmasa bile "bu nota kaç bağlantı
geliyor" cevabı değişmez.

### 6. `graph` — dışa aktarım

```console
$ nodemind --kasa .\ornek graph --bicim dot --dugum not-baslik
digraph dugumkafa {
  rankdir=LR;
  node [shape=box, fontname="monospace"];
  labelloc="t";
  label="NodeMind bilgi grafigi";
  "0f7ec25d2b8b" [label="Markdown Lehcesi (baslik) deg:0", type="baslik"];
  "14a0d0048538" [label="Notlar (baslik) deg:0", type="baslik"];
  ...
}
```

```console
$ nodemind --kasa .\ornek graph --bicim json --dugum not
{
  "dugumler": [
    {
      "id": "2139c9f8a5cf",
      "etiket": "Markdown Lehcesi",
      "tip": "not",
      "cikis": 0,
      "giris": 0
    },
    ...
  ],
  "kenarlar": [ ... ]
}
```

```console
$ nodemind --kasa .\ornek graph --bicim html --dugum not --cikti grafik.html
yazildi: .\ornek\grafik.html (html)
```

`--bicim html` üretilen sayfa **tamamen bağımsızdır**: JavaScript yoktur, dış
dosya veya CDN başvurusu yoktur; SVG sayfa içine gömülüdür. `--bicim ascii`
ile terminal çıktısı da dosyaya yazılabilir.

Göreli çıktı yolları **kasa köküne** göre çözülür; kasa dışına (`../...`)
çıkmak reddedilir.

### 7. `today` — günlük/haftalık akış

```console
$ nodemind --kasa .\ornek today --tarih 2026-09-29
# 2026-09-29
2026-09-29 — 29 Eylul 2026 Sali
dosya: gunluk/2026-09-29.md
---
  [madde] Ozet: Arama motorunu ve bilgi grafik ciktisini birlikte denedim.
  [madde] Baglantilar: [[Arama]], [[Bilgi Grafigi]]
  [madde] Kopuk kalan notlari yarini gunlune baglayacagim.
  [madde] Bir onceki gunun notu: [[gunluk/2026-09-28]]
```

```console
$ nodemind --kasa .\ornek today --tarih 2026-09-29 --gorunum hafta
ISO haftasi 40 (2026), 7 gun

  2026-09-28  2026-09-28
  2026-09-29  2026-09-29
  2026-09-30  (gunluk yok)
  2026-10-01  (gunluk yok)
  2026-10-02  (gunluk yok)
  2026-10-03  (gunluk yok)
  2026-10-04  (gunluk yok)
```

Günlük notu yoksa komut şablonu yazar ve **dosya oluşturmaz**:

```console
$ nodemind --kasa .\ornek today --tarih 2026-09-30
# 2026-09-30
2026-09-30 — 30 Eylul 2026 Carsamba
gunluk notu yok: 2026-09-30.md
---
# 2026-09-30

## Bugun

- Ozet: 
- Baglantilar: 

## Notlar

- 

olusturmak icin: nodemind today --bugun --yaz
```

`--yaz` eklenirse günlük **oluşturulur**:

```console
$ nodemind --kasa .\ornek today --tarih 2026-09-30 --yaz
# 2026-09-30
2026-09-30 — 30 Eylul 2026 Carsamba
gunluk notu yok: 2026-09-30.md
---
# 2026-09-30

## Bugun

- Ozet: 
- Baglantilar: 

## Notlar

- 
olusturuldu: .\ornek\2026-09-30.md
```

Aynı komut ikinci kez çalıştırıldığında günlük zaten vardır: **hiç değiştirilmez**,
mevcut içerik bloklarıyla gösterilir (`--yaz` idempotenttir).

```console
$ nodemind --kasa .\ornek today --tarih 2026-09-30 --yaz
# 2026-09-30
2026-09-30 — 30 Eylul 2026 Carsamba
dosya: 2026-09-30.md
---
  [madde] Ozet:
  [madde] Baglantilar:
  [paragraf] -
```

`nodemind new` ise farklı davranır: dosya varsa açık hata verir
(veri güvenliği; aşağıdaki `new` bölümüne bakınız).

### 8. `new` — yeni not (var olanı asla ezmez)

```console
$ nodemind --kasa C:\...\notlar new "Okuma Listesi" --baslik "Okuma Listesi"
olusturuldu: C:\...\notlar\Okuma Listesi.md
kasa: C:\...\notlar

$ nodemind --kasa C:\...\notlar new "Okuma Listesi"
hata: not zaten var, UZERINE YAZILMADI: C:\...\notlar\Okuma Listesi.md (mevcut not dosyalari asla degistirilmez)
```

### 9. `--help`

```console
$ nodemind --help
NodeMind, not klasorunu (.md dosyalari) okur; hicbir not dosyasina YAZMAZ. Blok
kimlikleri icerikten turetilir, indeks cache/index.json altinda tutulur ve
silinebilir.

Usage: nodemind.exe [OPTIONS] <COMMAND>

Commands:
  new        Yeni bir not dosyası oluşturur. Var olan bir not **asla** üzerine yazılmaz
  index      Not klasörünü tarar ve `cache/index.json` indeksini (yeniden) kurar
  search     Tam metin araması yapar (ters indeks; Türkçe normalizasyonlu)
  backlinks  Verilen nota veya bloğa **hangi blokların bağlandığını** listeler
  graph      Bilgi grafiğini çizer veya dışa aktarır
  today      Günlük/haftalık akış: bugünün (veya seçilen tarihin) günlüğünü gösterir
  check      Kırık referansları, yinelenen adları ve şema durumunu denetler
  help       Print this message or the help of the given subcommand(s)

Options:
  -v, --kasa <YOL>
          Not klasörü (kasa). Verilmezse sırayla: ./notlar, yürütülebilir yanı/notlar, çalışma dizini

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

---

## Test

```console
$ cargo test
   Compiling nodemind v0.1.0 (C:\...\projects\21-nodemind)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 24.62s
     Running unittests src\lib.rs (target\debug\deps\nodemind-2038f3c52b3b6d01.exe)
running 246 tests
test result: ok. 246 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s

     Running unittests src\main.rs (target\debug\deps\nodemind-931f152fb9ccbaee.exe)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\cli_entegrasyon.rs (target\debug\deps\cli_entegrasyon-5df19c8a1cc1e0a.exe)
running 24 tests
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s

     Running tests\entegrasyon.rs (target\debug\deps\entegrasyon-2e192aa8aa3c3605.exe)
running 17 tests
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

   Doc-tests nodemind
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Test sonucu: 287 okunan, 0 başarısız** (246 birim + 24 CLI entegrasyon +
17 kütüphane entegrasyon). Kapsanan konular:

| Konu | Örnek testler |
|---|---|
| İç içe liste ayrıştırma | `ic_ice_liste_maddeleri_agac_olusur`, `liste_devam_satiri_maddenin_cocugu_olur` |
| Başlık hiyerarşisi | `baslik_higherasi_agac_olusur`, `yollar_ve_derinlikler_tutarli` |
| Boş not / boş dosya | `bos_not_hicbir_blok_uretmez`, `bos_not_dosyasi_taranir_ama_blok_uretmez` |
| Blok kimliği kararlılığı | `blok_kimligi_aynistirmada_ayni_kalir`, `blok_kimligi_not_id_ile_degisir` |
| Kimlik çakışması | `ayni_metni_iki_kez_icen_bloklar_farkli_kimlik_alir` |
| `[[...]]` / `((...))` çözümleme | `tek_cift_koseli_parantez_cikarilir`, `etiketli_baglantida_ad_kismi_alinir` |
| Yinelenen ad belirsizliği | `yinelenen_ad_belirsiz_isaretlenir`, `rapor_yinelenen_adi_belirsiz_olarak_listeler` |
| Kırık referans | `kirik_not_baglantisi_isaretlenir`, `rapor_kirk_baglantiyi_listeler` |
| Kendine bağlantı | `kendine_blok_baglantisi_yakalnir`, `kendine_not_baglantisi_yakalnir` |
| Ters indeks / ters bağlantı | `ters_baglantilar_not_bazinda_calisir`, `silme_terimi_indensten_kaldirir` |
| Türkçe karakter arama | `turkce_karakterle_yazilan_sorgu_ascii_yazimiyla_bulur`, `katlama_turkce_harfleri_asciiye_indirger` |
| Büyük/küçük harf | `buyuk_kucuk_harf_duyarsiz_arama_yapar` |
| Arama kombinasyonları | `cok_terimli_sorgu_hepsini_isteyen_bloklari_bulur`, `coklu_terim_puanlari_toplanir` |
| Alan ağırlığı sıralaması | `alan_agirligi_basligi_one_almaya_ettirir`, `tur_suzgeci_calisir` |
| Kök indirgeme / yumuşama | `kok_indirgeme_ekleri_soker`, `kok_indirgeme_unsuz_yumusatmasi_uygular` |
| ASCII grafik — düğümsüz | `ascii_bos_graf_mesaji_yazar` |
| ASCII grafik — tek düğüm | `ascii_tek_dugum_tek_bilesen_yazar` |
| ASCII grafik — döngü | `ascii_donguyu_cevrim_satirinda_gosterir`, `iki_dugumlu_dongu_kirilir` |
| ASCII grafik — kopuk bileşen | `ascii_kopuk_bilesenleri_numaralandirir`, `kopuk_bilesenler_ayri_ayri_cizilir` |
| ASCII grafik — kök seçimi | `ascii_kok_dugumu_yalnizca_bir_kez_basar`, `ascii_koku_not_dugumleri_arasindan_secer` |
| DOT dışa aktarımı | `dot_dosyasi_gecerli_gorunur`, `dot_yapı_kenarlarini_kesikli_cizer` |
| HTML/SVG dışa aktarımı | `html_sayfasi_bagimsiz_ve_dogru_uretir`, `html_etiketleri_kacirir` |
| JSON şema gidiş-dönüşü | `json_gidis_donusu_kimliktir`, `json_gidis_donusu_iki_kez_ayni_metni_uretilir` |
| Sürüm/şema uyuşmazlığı | `yukle_uyumsuz_sema_dosyasinda_hata_verir`, `uyumsuz_sema_yuklemede_hata_dondurur` |
| Günlük akış | `hafta_gunleri_yedi_gun_dondurur`, `iso_hafta_numarasi_known_vektorlarla_dogru` |
| Tarih test vektörleri | `epoch_sifir_yirminci_yuzyildir`, `donusum_cift_yonlu_kararlidir`, `gercek_tarihler_dogru_hesaplanir` |
| Yol kaçışı | `yol_kacisi_reddedilir`, `kasa_disi_yollar_reddedilir`, `yeni_not_yeni_not_olusturur` |
| Veri güvenliği | `var_olan_not_uzerine_yazilmaz`, `yeni_not_var_olan_dosyayi_ezmez`, `today_yaz_ile_yeni_gunluk_olusturur` |
| BOM dayanıklılığı | `bom_ilk_basligi_bozmaz` |
| Çok baytlı karakter | `turkce_karakterli_hedef_korunur` (bayt dilimleme yerine `char` taraması) |

Kalite kapısının diğer üç kolu:

```console
$ cargo clippy --all-targets -- -D warnings
    Checking nodemind v0.1.0 (C:\...\projects\21-nodemind)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.12s

$ cargo fmt --check
$ echo $?
0
$ cargo build --release
    Finished `release` profile [optimized] target(s) in 17.97s
```

---

## Proje Yapısı

```
projects/21-nodemind/
├── Cargo.toml
├── Cargo.lock
├── LICENSE.txt
├── README.md
├── .gitignore
├── .github/workflows/ci.yml
├── ornek/                        örnek not koleksiyonu (10 not, iki lehçe bağlantısı)
│   ├── Arama.md
│   ├── Bilgi Grafigi.md
│   ├── Markdown Lehcesi.md
│   ├── bos.md
│   ├── kopuk-not.md
│   ├── arsiv/2026-09-01.md      yinelenen ad demosu
│   ├── gunluk/2026-09-01.md
│   ├── gunluk/2026-09-28.md
│   ├── gunluk/2026-09-29.md
│   └── notlar/calisma-duzeni.md
├── src/                         5 754 satır (üretim) + 65 satır (test yardımcısı)
│   ├── lib.rs                   çekirdek kütüphane, indeks kurulumu (203)
│   ├── main.rs                  CLI kabuğu; yalnızca ayrıştırma + çıktı (749)
│   ├── hata.rs                  hata tipleri + elle Display/Error (245)
│   ├── kimlik.rs                FNV-1a 64 ve kararlı blok kimliği (147)
│   ├── markdown.rs              kendi Markdown ayrıştırıcımız → blok ağacı (931)
│   ├── referans.rs              [[...]] / ((...)) çıkarma ve çözümleme (578)
│   ├── sema.rs                  cache/index.json şeması ve sürüm denetimi (313)
│   ├── indeks.rs                ters indeks, Türkçe normalizasyon, arama (610)
│   ├── kasa.rs                  kasa taraması, not okuma, yeni not oluşturma (658)
│   ├── gunluk.rs                sivil tarih (std::time) + günlük/hafta şablonu (439)
│   ├── cozum.rs                 kasa → düzce → rapor ve ters bağlantılar (277)
│   ├── graf.rs                  graf modeli, ASCII çizim, JSON/DOT/HTML-SVG (1 039)
│   └── test_yardimcisi.rs       yalnızca testlerde geçici dizin, Drop ile (65)
└── tests/                       676 satır
    ├── yardimci/mod.rs          entegrasyon testi geçici kasa + ikili çalıştırıcı (108)
    ├── entegrasyon.rs           modüller arası akış, 17 test (305)
    └── cli_entegrasyon.rs       derlenmiş ikilinin davranışı, 24 test (263)
```

Toplam **6 930 satır Rust** (üretim 5 754 + testler 1 176). Tek dosya ikilisi
`target/release/nodemind.exe` **1 509 662 bayt** (Windows x64, `--release`,
`strip = "debuginfo"` ile).

---

## Yapılandırma

NodeMind **yapılandırma dosyası okumaz**; tüm ayarlar bayrakla verilir. Bu,
raporun "USB'den çalışma" gereksinimi için bilinçli bir tercihtir: yanında
`config.toml` olmayan tek dosya, yanında yanlış dosya olan tek dosyadan iyidir.

| Bayrak | Varsayılan | Etkisi |
|---|---|---|
| `-v, --kasa <YOL>` | `./notlar`, sonra `<exe yanı>/notlar` | Not klasörü (kasa) kökü. Verilmezse bu sırayla aranır, hiçbiri yoksa hata verilir. |
| `new --baslik <METIN>` | not adı | Yeni notun `#` başlığı. |
| `new --icerik <METIN>` | `- ` (boş madde) | Yeni notun gövdesi. |
| `search --tur <TUR>` | yok | `baslik`, `madde`, `alinti`, `kod`, `paragraf`. Verilmezse tüm türler. |
| `search --not <AD>` | yok | Not adı veya göreli yol. Bulunamazsa uyarı verilir, filtre uygulanmaz. |
| `search --limit <N>` | `20` | Gösterilecek en fazla sonuç. `0` = sınırsız. |
| `backlinks --tur <TUR>` | yok | `not` (yalnız `[[...]]`) veya `blok` (yalnız `((...))`). |
| `graph --bicim <BICIM>` | `ascii` | `ascii`, `json`, `dot`, `html`. |
| `graph --dugum <SECIM>` | `hepsi` | `not`, `not-baslik`, `hepsi`. |
| `graph --cikti <DOSYA>` | stdout | Göreli yollar kasa köküne göre çözülür; kasa dışı reddedilir. |
| `graph --en-cok <N>` | `5` | ASCII çıktısında listelenecek en çok bağlantılı not sayısı. `0` = gösterme. |
| `today --bugun` | bugün | `--bugün` takma adı da vardır. `--tarih` ile **birlikte kullanılamaz**. |
| `today --tarih <TARIH>` | bugün | `bugun`, `YYYY-AA-GG` veya `YYYY-AA`. |
| `today --gorunum <GORUNUM>` | `gun` | `gun` veya `hafta` (ISO haftasının 7 günü). |
| `today --yaz, -y` | yok | Günlük yoksa şablonla **oluşturur**. Varsa hiç değiştirmez. |
| `check --json` | yok | Makine-okunur JSON raporu. |

Türetilen dosya: `<kasa>/cache/index.json` (şema sürümü `1`).

---

## Bilinen Sınırlamalar

Bu bölüm **dürüst olmak için** yazıldı; hiçbir madde ölçülmüş değildir.

### Ertelenen (MANIFEST kart 21 "Ertelenen" listesi)

* **Günlük/haftalık akışın tamamı.** Bu depoda yalnızca tarihli not okuma,
  ISO haftası listesi ve günlük oluşturma vardır. Otomatik açılış, haftalık
  dönüşüm sayfası ve **yıl sonu arşivleme yoktur**.
* **Arşivleme.** Sıkıştırılmış yıllık arşiv ve geri alma uygulanmamıştır.
* **Dosya sistemi senkronizasyonu.** NodeMind yalnızca okur; senkronizasyon
  kullanıcının kendi aracına (Syncthing, git, bulut sürücüsü) bırakılmıştır.
* **Şifreli depo.** Kapsül modu yoktur; notlar düz metindir.
* **Grafik arayüz (GUI).** Yalnızca terminal ve statik HTML/SVG. Raporun `egui`
  önerisi yerine `WORKER_CONTRACT.md` § 3.2-G ile pencere katmanı tamamen
  dışlanmıştır (taşınabilirlik + bellek gerekçesi).
* **Zengin metin düzenleme.** NodeMind bir **okuyucu ve arama motorudur**;
  düzenleyici yoktur. Yazma işlemi yalnızca yeni dosya oluşturmaktır.
* **Otomatik bağlama önerileri** (`[[` yazınca ad benzerliğiyle öneri listesi).
* **Logseq blok kimliği biçimiyle uyum.** `((kimlik))` sözdizimi Logseq tarzıdır
  ama kimlik üretim kuralımız farklıdır; Logseq'in yazdığı kimlikler bizim
  indeksimizde kırık görünür.

### Markdown lehçesi

* Tablolar, satır kırılmaları, satır içi biçimlendirme (kalın/italik), bağlantı
  referans tanımları (`[metin][etiket]`) ve HTML blokları **ayrıştırılmaz**;
  metin olduğu gibi paragraf olarak saklanır.
* Setext başlıkları (`Başlık\n===`) desteklenmez.
* Alıntı satırları bir düzey soyulur ve alt metin yeniden ayrıştırılır; bu
  yüzden `>> metin` iç içe iki alıntı üretir.

### Blok kimliği — **kasıtlı** bir davranış

Blok kimliği içerikten ve konumdan türetilir. Bunun sonucu:

* Bir bloğun **metni değişirse** kimliği değişir ve o bloğa yazılmış
  `((kimlik))` referansları `check` çıktısında **kırık** olarak listelenir.
* Bir bloğun **üstüne yeni bir kardeş eklenirse** (ağaç yolu `0.1` → `0.2`)
  kardeşlerin kimlikleri kayabilir.
* Kimlik **dosyaya yazılmaz**. Bu, "not dosyaları asla yazılmaz" kuralıyla
  çelişmemek için seçilmiştir; Logseq'in yaptığı gibi `^ id` işaretçisi
  yazmamak, kullanıcının dosyalarının başka araçlarla da temiz kalmasını
  sağlar. Bedeli, yukarıdaki iki maddedir.
* 12 onaltılık karakter (48 bit) kullanılır; 10.000 notluk bir kasa için
  çakışma olasılığı ihmal edilebilir düzeydedir, ama ** garanti değildir.
  Çakışma olursa `-2`, `-3` eklenerek ayrıştırma sırasıyla çözülür.

### Arama

* Kök indirgeme **kural tabanlıdır**; Morfeus/Porter gibi bir lemmatizer
  değildir. Ünlülenen ünsüzler (çocuk → çocuk değil) kapsam dışıdır.
* Aksanlı Latin harfler (é, ñ) katlanmaz; `é` yazan bir kullanıcı `e` ile
  bulamaz.
* Sorgular **VE** mantığındadır; OR, joker karakter ve alan içi ifade yoktur.
* Arama sonucu önizlemesi bloğun ilk 160 karakteridir; tam metin için `backlinks`
  ve `graph` çıktısındaki blok kimliğini kullanın.
* `graph --dugum not` seçildiğinde **bağlantı kenarları çizilmez** (bağlantı
  blok düğümlerinden nota gider ve blok düğümleri elenir). Derece değerleri
  yine de doğrudur; yalnızca kenarları görmek için `not-baslik` veya `hepsi`
  seçilmelidir.

### Çakışma ve eşzamanlılık

* NodeMind **kilit dosyası kullanmaz** ve notlara yazmaz. İki araç (siz ve
  Obsidian) aynı dosyayı aynı anda düzenlerse **son yazan kazanır**; NodeMind
  bir sonraki taramada dosyayı diskten yeniden okur ve tutarsız bir durum
  göstermez. Veri kaybı yazma tarafındadır, okuma tarafında değil.
* `search` komutu önbelleği kullanır; bir not arada değişirse (dosya zaman
  damgası farklı) indeks bayat sayılır, bellekte yeniden kurulur ve stderr'a
  uyarı yazılır.

### Ölçülmemiş sayılar

* Raporun bellek bütçesi (260 MB / 10.000 not), soğuk açılış süresi (1,2 s) ve
  indeks boyutu (180 MB) hedefleridir. **Bu depoda ölçülmemiştir** ve hiçbir
  yerde sayı olarak verilmemiştir. Tek ölçülen sayı ikili dosya boyutudur.
* Segmentli indeks, disk üzerinde indeks ve arama belleği sabit tutma
  uygulanmamıştır; indeks bellekte `BTreeMap` olarak tutulur.

### Diğer

* Aynı adı taşıyan iki not varsa `[[ad]]` **belirsiz** sayılır ve hiçbir
  ters bağlantı üretilmez. `check` adayları listeler; notlardan birini
  yeniden adlandırmak sorunu çözer.
* `--bicim html` çıktısı **dairesel** bir düzen kullanır. Büyük kasa (yüzlerce
  düğüm) için okunabilir bir yerleşim değildir; amacı hızlı bir önizlemedir.
* Terminal çıktısı **ASCII**'dir; Unicode kutu çizim karakteri kullanılmaz.
* `Drop` içindeki geçici dizin temizliği hataları bilinçli olarak yutulur
  (`let _ = ...`); `Drop` içinden hata döndürülemez. Bu, sözleşmenin "sessiz
  yutma" yasağına yegdir ve yalnızca test yardımcısında geçerlidir.

---

## Gelecek Geliştirmeler

1. **Arşivleme** — seçili yılın notlarını tek bir Markdown arşivine taşımak ve
   geri alınabilir işlem. `MANIFEST.md` kart 21'de ertelenen ilk madde.
2. **Günlük/haftalık şablon yönetimi** — kullanıcıya özel şablon dosyası ve
   haftalık dönüşüm sayfası üretimi.
3. **Segmentli indeks** — indeks bellekte tutulduğu için 10.000 notun üzerinde
   bellek bütçesi aşılabilir; terim çalışma kümesi veya diske yazılan segment
   indeks doğal sonraki adımdır.
4. **Tam metin aramada alan içi ifade** — `"tırnaklı bölüm"`, `etiket:#baslik`,
   `yol:gunluk/` gibi süzgeçler.
5. **Graf düzen iyileştirmesi** — katmanlı yerleşim (Sugiyama basit sürümü)
   ve SVG dışa aktarımında gerçek yönlendirme; şu an dairesel düzen kullanılıyor.
6. **Yol bağımsızlığı sertleştirmesi** — `current_exe` ile yürütülebilir
   konumundan kasa çözümlemesi zaten var; USB'de salt okunur bağlanmış diskte
   `cache/` yazımı için yalnızca-okunur kip uyarısı eklenebilir.
7. **Logseq uyumluluğu** — Logseq'in `^ id` işaretçilerini **okuyup** kendi
   indeksimizde tanımak (yazmaya çalışmadan).

---

## Troubleshooting

### 1) `hata: not zaten var, UZERINE YAZILMADI: ...`

**Belirti:** `nodemind new "X"` komutu hata döndürüyor.
**Neden:** `X.md` zaten var. Bu bir hile değil, veri güvenliği kuralıdır:
NodeMind mevcut not dosyalarını **asla** değiştirmez.
**Çözüm:** Var olan notu bir metin düzenleyicide açıp düzenleyin. Yeni bir not
istiyorsanız farklı bir ad verin (`X 2`, `X taslak`). Aynı davranış
`today --yaz` için de geçerlidir: günlük varsa **gösterilir**, değiştirilmez.

### 2) `hata: indeks semasi uyusmuyor: ... (dosyada 2, beklenen 1)`

**Belirti:** `search` veya `check` uyarı yazıyor, `check` "UYUMSUZ" diyor.
**Neden:** `cache/index.json` dosyası başka bir NodeMind sürümü tarafından
yazılmış. İndeks **türetilmiş veridir**, sürüm değişince yeniden üretilir.
**Çözüm:** `nodemind --kasa <kasa> index`. Alternatif olarak
`cache/index.json` dosyasını silmek de yeterlidir — not dosyalarına dokunulmaz.
`search` zaten bu durumda bellekte yeniden kurup uyarı yazar; yalnızca `check`
çıkış kodu `2` döner.

### 3) `eslesme yok: <sorgu>` ama metin dosyada görünüyor

**Belirti:** Arama boş dönüyor.
**Nedenlar ve çözümler:**
* Sorgu **birden çok terim** içeriyorsa hepsi aynı blokta bulunmalıdır
  (VE mantığı). Tek tek deneyin: `nodemind search "harf"` yerine
  `nodemind search "harf katlamasi"` de ikisini de arar.
* Türkçe harfler ASCII'ye katlanır ama **aksanlı Latin harfler katlanmaz**.
  `é` yazan bir metni `e` ile bulamazsınız.
* Sorgu bir **kod bloğu** içindeyse `search --tur kod` ile arayın; kod blokları
  en düşük alan ağırlığına sahiptir ve normal sıralamada aşağıda kalır.
* `cache/index.json` bayattır ve konsolun **stderr** kanalına uyarı yazılır.
  `nodemind index` çalıştırın.

### 4) `[[Not Adı]]` çözülmüyor ama dosya adı birebir aynı

**Belirti:** `check` çıktısında `belirsiz` ya da `kirik` listeleniyor.
**Neyse göre çözülür:** Not, dosya adı (uzantısız) ve kasa köküne göreli yol
**iki biçimde** kaydedilir; karşılaştırma ASCII'ye katlanmış ve büyük/küçük
harf duyarsızdır. `[[Markdown Lehcesi]]` ile `[[Markdown Lehcesi.md]]` çalışır.
**Çözüm:** `check` çıktısındaki `yinelenen adlar` listesine bakın. İki not aynı
adı taşıyorsa bağlantı **belirsizdir**; notlardan birini yeniden adlandırın
veya bağlantıyı göreli yolla yazın (`[[gunluk/2026-09-28]]`). Dosya uzantısı
`.md` veya `.markdown` olmalıdır; `.txt` dosyaları taranmaz.

### 5) `hata: gecersiz secenek kombinasyonu: --bugun --tarih`

**Belirti:** `today` komutu hata veriyor.
**Neden:** `--bugun` ve `--tarih` aynı anda verilmiş. "Bugün" ile "belirli bir
tarih" aynı anda seçilemez.
**Çözüm:** `nodemind today --bugun` **veya** `nodemind today --tarih
2026-09-29`. `--bugün` (Türkçe) takma adı da kabul edilir.

### 6) `graph` çıktısı devasa / okunmuyor

**Belirti:** Yüzlerce satırlık ASCII ağaç.
**Neden:** Varsayılan `--dugum hepsi` her bloğu düğüm yapar.
**Çözüm:** `--dugum not-baslik` (not + başlıklar) veya `--dugum not`. Derece
bilgisi gerekiyorsa `not-baslik` gereklidir, çünkü bağlantı kenarları blok
düğümlerinden nota gider.

---

## Atıflar

Bu proje **hiçbir harici kütüphane kopyalamaz**; aşağıdakiler ya kullanılan
API'lerin resmî belgeleridir ya da tasarım kararlarının kaynağıdır.

* **Rust standart kütüphane** — `std::fs`, `std::collections`, `std::time`,
  <https://doc.rust-lang.org/std/>
* **Rust 2021 sürüm rehberi** — <https://doc.rust-lang.org/edition-guide/edition-2021.html>
* **`cargo` yerel rehberi** — <https://doc.rust-lang.org/cargo/>
* **`serde`** — türetilmiş veri yapıları, <https://serde.rs/>
* **`serde_json`** — JSON okuma/yazma, <https://github.com/serde-rs/json>
* **`clap`** — komut satırı argüman ayrıştırma, <https://docs.rs/clap/>
* **FNV-1a hash** — A. S. Jenkins, *Fast Hashing* (1997),
  <https://www.azillionmonkeys.com/qed/hash.html>
* **Sivil tarih dönüşümü** — Howard Hinnant, *chrono-Compatible Low-Level Date
  Algorithms*, <https://howardhinnant.github.io/date_algorithms.html>
* **ISO 8601 hafta numarası** — <https://www.iso.org/iso-8601-date-and-time-format.html>
* **Markdown lehçesi** — CommonMark spesifikasyonu (kapsam dışı bırakılan
  yapılar için referans), <https://spec.commonmark.org/>
* **Graphviz DOT dili** — <https://graphviz.org/doc/info/lang.html>
* **Logseq blok referans lehçesi** — `((kimlik))` sözdizimi,
  <https://github.com/logseq/logseq>
* **Obsidian `[[bağlantı]]` lehçesi** — <https://help.obsidian.md/Linking+notes+and+files>
* **Türkçe kök indirgeme** — aşağıdaki ek listesi ve ünsüz yumuşaması kuralı,
  dilbilimsel bir makaleye değil **yaygın eklerin gözlemsel listesine**
  dayanır; bu nedenle bir "stemmer" olarak değil, "basit normalleştirici" olarak
  sunulur ve `Bilinen Sınırlamalar`da bu sınır açıkça yazılıdır.
* **Türkiye Cumhuriyeti Bilgi ve İletişim Kurumu (BTK) — karakter kodlaması** —
  <https://www.btk.gov.tr/>
* **Rapor dosyası (iç tasarımın kaynağı)**:
  `%USERPROFILE%\Desktop\Fikirler\21-dugum-kafa-notlar.html` — yerel dosyadır,
  çevrimiçi bir karşılığı yoktur. Bölümler b01 (yönetici özeti), b03 (hedef
  kitle ve kabul kriterleri), b05 (özellik seti), b07 (teknik tasarım),
  b08 (bellek bütçesi), b09 (taşınabilirlik) ve b16 (açık sorular) bu
  uygulamanın kapsamını belirlemiştir.

---

## Üretim Atfı

Bu depo **OpenCode** ajanı tarafından, **`space-bunny-free`** modeli
(`opencode/space-bunny-free`) kullanılarak üretilmiştir.

- **Arac:** OpenCode
- **Model:** `opencode/space-bunny-free` (Space Bunny Free)
- **Tür:** Rust, `cargo build` / `cargo test` ile üretilmiş ve doğrulanmıştır.

Kaynak kod, testler ve dokümantasyon bu model tarafından yazılmıştır. İnsan
katkısı: gereksinim tanımı, kabul ölçütleri ve son kontroller.

## Lisans

MIT — tam metin `LICENSE.txt` dosyasındadır.

Bu proje raporda GPL-3.0-or-later ile öngörülmüştür; **MIT** ile dağıtılır.
Harici kütüphanenin kopyalanmamış olması nedeniyle bu bir lisans çelişkisi
değildir (bkz. `MANIFEST.md` kart 21, karar D-003).
