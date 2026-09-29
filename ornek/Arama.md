# Arama

Gomulu tam metin aramasi: kendi ters indeksimiz, FTS kütüphanesi yok.

## Kriterler

- Turkce harf katlamasi yapilir: `I/ı/İ/i` -> `i`, `Ş/s` -> `s`, `Ğ/g` -> `g`, `Ü/u` -> `u`, `Ö/o` -> `o`, `Ç/c` -> `c`.
- Basit kok indirgeme uygulanir ve **unsuz yumusamasi** eklenir (`kitabın` -> `kitap`).
- Sorgu terimleri **VE** ile birlestirilir; en az bir terimi bulamayan blok sonuclarda yer almaz.
- Siralama olcutu: alan agirligi (baslik > madde > alinti > paragraf > kod) ve terim frekansi.

## Indeks

Indeks `cache/index.json` altinda tutulur. Sema surumu uymuyorsa dosya kullanilmaz,
acik bir hata doner ve `nodemind index` ile yeniden kurulur. Indeks **turetilmis
veridir**: silinebilir ve notlardan yeniden uretilebilir.

[[Markdown Lehcesi]] ile baglantilidir.