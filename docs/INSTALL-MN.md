# ChatNinja — суулгах ба эхний тохиргоо

## Одоогийн хувилбар

0.1.0 бол хөгжүүлэлтийн хувилбар. Windows installer бүтээх, суулгах, асаах,
хаах, устгах шалгалт GitHub Actions-д бий. Тухайн build-ийн бодит үр дүнг
Actions болон VERIFICATION.md-ээс харна. Бүх тоглоом, бүх account дээр
туршсан нийтэд түгээх эцсийн хувилбар гэж үзэж болохгүй.

## Суулгаж ашиглах

1. Installer `.exe` файлыг ажиллуулж суулгана. Source ZIP нь installer биш.
2. ChatNinja-г нээнэ. Анх удаа загвар чаттай DEMO горим харагдана.
3. Overlay хэсэгт хэмжээ, байрлал, үсэг, дэвсгэрийн тунгалаг байдлыг тохируулна.
4. Дэлгэцийн overlay-г нээж, түгжээ тайлагдсан үед чирж байрлуулна.
5. Click-through асаахад хулганын даралт доорх тоглоомд дамжина.
6. `Alt+Shift+O` харуулах/нуух, `Alt+Shift+L` click-through солих товчлол.
   Товчлол давхацвал app мэдэгдэнэ; үндсэн цонхны удирдлагыг ашиглана.
7. OBS дээр гаргах бол OBS эсвэл хосолсон горим сонгож, тухайн локал URL-ийг
   OBS Browser Source-д хуулна. App дахин асахад URL өөрчлөгдөнө.

Үндсэн цонхыг хаахад app, overlay болон chat холболт зогсоно. Одоогоор
tray горимгүй. Exclusive fullscreen үед overlay далдлагдаж болно;
windowed/borderless горимоор шалгана. Streamer-only горимын capture protection
нь бүх capture арга дээр баталгаатай биш тул бичлэг хийж шалгана.

Installer одоогоор code-sign хийгдээгүй. Windows анхааруулга үзүүлж болно.
Татсан файл зөв эсэхийг хамт гарсан SHA-256-тай харьцуулж болно.

## Live чат яагаад шууд идэвхгүй байж болох вэ?

App-ийг түгээгч нь Google, Twitch developer app болон Kick серверээ нэг удаа
тохируулах шаардлагатай. Энэ нь энгийн streamer бүрээр хийлгэх тохиргоо биш.
Тохиргоогүй build дээр Connect идэвхгүй бөгөөд шалтгааныг харуулна.

GitHub repository → Settings → Secrets and variables → Actions:

| Төрөл | Нэр | Утга |
| --- | --- | --- |
| Variable | `CHATNINJA_GOOGLE_CLIENT_ID` | Google **Desktop app** OAuth client ID |
| Secret | `CHATNINJA_GOOGLE_DESKTOP_SECRET` | Тухайн Google desktop client-ийн утга, шаардлагатай бол |
| Variable | `CHATNINJA_TWITCH_CLIENT_ID` | Twitch **Public** developer app client ID |
| Variable | `CHATNINJA_KICK_RELAY_ORIGIN` | Тохируулсан Kick relay-ийн HTTPS үндсэн хаяг |

Google төсөлд YouTube Data API v3-г идэвхжүүлж, consent/test-user тохиргоог
хийсэн байна. Kick-ийн client secret болон encryption key нь зөвхөн серверийн
secret manager-д байна. Нууц үг, token, client secret-ийг чат эсвэл source code-д
бичихгүй. Дэлгэрэнгүй заавар: PROVIDER-SETUP.md болон services/kick-relay/README.md.

Тохируулсны дараа Actions → Quality checks → Run workflow ажиллуулж шинэ
installer бүтээнэ. Streamer Channels хэсгээс Connect дарж browser дээр
зөвшөөрлөө өгнө. YouTube/Twitch өөрийн сувгийн чат, Kick өөрийн баталгаажсан
сувгийн webhook чат ашиглана. Нэвтрэхэд DEMO унтарна.

## Дуусаагүй зүйлс

Бодит гурван платформын end-to-end туршилт, Kick deployment, бүх emote catalogue,
Windows 10/11 болон тоглоом/OBS туршилт, signed release үлдсэн. Twitch native
emote fragment-ийн код байгаа; бүх native болон 7TV/BTTV/FFZ emote ажиллана гэж
одоогоор амлахгүй.
