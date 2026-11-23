# История изменений / Changelog — ReStem

## [v1.4.0] — 2026-11-23
### 🇷🇺 Что нового
* **Поддержка MediaInfo для переименования видео:**
  * Добавлены новые умные теги для переименования видеофайлов: `{res}` (`2160p`, `1080p`, `720p`, `480p` и др.), `{vcodec}` (`H264`, `HEVC`, `AV1`, `VP9`, `ProRes` и др.), `{acodec}` (`AAC`, `AC3`, `EAC3`, `Opus`, `DTS`, `FLAC` и др.), `{ach}` (`1.0`, `2.0`, `5.1`, `7.1`), `{fps}` (`23.98fps`, `24fps`, `60fps`), `{duration}` (`04m35s`, `01h02m05s`), `{bitrate}` (`15Mbps`, `850kbps`), `{hdr}` (`HDR10`, `HLG`, `DV`) и `{media_date}` (дата записи из метаданных контейнера).
  * Существующие теги `{w}x{h}`, `{w}`, `{h}` теперь также автоматически работают для видеофайлов.
* **Встроенный парсер контейнеров + внешний fallback:**
  * Нативное чтение заголовков на чистом Rust (без внешних DLL и зависимостей) для `MP4`, `M4V`, `MOV`, `3GP`, `MKV`, `WebM`, `AVI` и `FLV`.
  * При наличии в `PATH` системных утилит `MediaInfo CLI` или `ffprobe` они автоматически используются как запасной источник для редких форматов (`TS`, `M2TS`, `WMV`, `MPG`, `VOB`, `OGV`) или неполных заголовков.
* **Предпросмотр MediaInfo и быстрые пресеты:**
  * В карточке **«👁 Предпросмотр объекта»** при выборе видео отображаются разрешение, класс качества, HDR, видео-/аудиокодеки, каналы звука, длительность, битрейт, дата записи и источник метаданных.
  * В таблице добавлена визуальная пометка `🎬 Видео` в столбце **«Тип»**, а на панели правил и пресетов — быстрая вставка видео-тегов в один клик и готовые пресеты для переименования фильмов и роликов.

### 🇬🇧 What's New
* **Video MediaInfo Support for Batch Renaming:**
  * Added new Smart Tags for video files: `{res}` (`2160p`, `1080p`, `720p`, `480p`, etc.), `{vcodec}` (`H264`, `HEVC`, `AV1`, `VP9`, `ProRes`, etc.), `{acodec}` (`AAC`, `AC3`, `EAC3`, `Opus`, `DTS`, `FLAC`, etc.), `{ach}` (`1.0`, `2.0`, `5.1`, `7.1`), `{fps}` (`23.98fps`, `24fps`, `60fps`), `{duration}` (`04m35s`, `01h02m05s`), `{bitrate}` (`15Mbps`, `850kbps`), `{hdr}` (`HDR10`, `HLG`, `DV`), and `{media_date}` (recorded date from container metadata).
  * Existing `{w}x{h}`, `{w}`, `{h}` dimension tags now work automatically for video files as well.
* **Built-In Container Parser + Optional External Fallback:**
  * Zero-dependency pure-Rust header parsing for `MP4`, `M4V`, `MOV`, `3GP`, `MKV`, `WebM`, `AVI`, and `FLV`.
  * Automatically falls back to `MediaInfo CLI` or `ffprobe` (if available in `PATH`) for uncommon containers (`TS`, `M2TS`, `WMV`, `MPG`, `VOB`, `OGV`) or incomplete headers.
* **Video MediaInfo Preview & Quick Presets:**
  * Selecting a video file in **👁 Item Preview** displays resolution, quality tier, HDR status, video/audio codecs, channels, duration, bitrate, recorded date, and parser source.
  * Added a distinct `🎬 Video` type badge in the preview table, one-click video tag buttons in Section 3, and dedicated quick presets for batch renaming videos.

---

## [v1.3.0] — 2026-11-14
### 🇷🇺 Что нового
* **Умные теги (Smart Tags) и чтение EXIF:** динамическая подстановка переменных `{date}`, `{year}`, `{month}`, `{day}`, `{exif_date}` (дата съёмки из EXIF), `{camera}` (модель камеры/смартфона из EXIF), `{parent}` (имя родительской папки) и `{w}x{h}` (разрешение изображения в пикселях).
* **Новые правила переименования:** двусторонняя транслитерация (`RU → LAT` и обратная `LAT → RU`), функция санации имён (автоочистка двойных пробелов и недопустимых символов), вставка текста по индексу символа (от начала или с конца) и переименование по списку (`.txt` / `.csv` / буфер обмена).
* **Посимвольная Diff-подсветка и масштабирование UI:** наглядное выделение удаляемых и добавляемых символов прямо внутри имён файлов в таблице, встроенная интерактивная шпаргалка по регулярным выражениям (`[?] Regex`) и кнопки масштабирования интерфейса (`A-` / `100%` / `A+`).
* **Системная интеграция и многошаговый откат:** стек истории откатов на 20 шагов (`Ctrl+Z`), экспорт автономного `.bat`-скрипта отката, открытие файлов двойным кликом, автосохранение настроек и пользовательских пресетов, поддержка аргументов командной строки и добавление пункта «Открыть в ReStem» в контекстное меню папок Проводника Windows.
* **Интеграция с GitHub:** добавлена кнопка быстрого перехода в репозиторий `🌐 GitHub` на верхней панели инструментов, а также прямые ссылки на репозиторий и страницу проверки новых релизов (`Releases`) в окне «ℹ О программе».

### 🇬🇧 What's New
* **Smart Tags & Built-In EXIF Reader:** dynamic tag expansion for `{date}`, `{year}`, `{month}`, `{day}`, `{exif_date}` (JPEG EXIF capture date), `{camera}` (EXIF camera model), `{parent}` (parent folder name), and `{w}x{h}` (image dimensions in pixels).
* **New Renaming Rules:** bidirectional transliteration (`RU → LAT` and reverse `LAT → RU`), filename sanitization (collapsing multiple spaces and stripping invalid characters), character-index text insertion, and batch renaming from custom lists (`.txt` / `.csv` / clipboard).
* **Inline Character Diff & UI Scaling:** real-time character-level diff highlighting in the preview table, interactive Regex Cheat Sheet (`[?] Regex`), and UI zoom controls (`A-` / `100%` / `A+`).
* **System Integration & Multi-Step Undo:** 20-step Undo history stack (`Ctrl+Z`), standalone `.bat` rollback script export, double-click file opening, persistent settings and custom user presets, CLI folder path arguments, and 1-click Windows Explorer folder context menu integration ("Open in ReStem").
* **GitHub Repository Integration:** added a quick `🌐 GitHub` button in the top toolbar and direct links to the official repository and latest `Releases` page inside the `ℹ About` dialog.

---

## [v1.2.0] — 2026-11
### 🇷🇺 Что нового
* **Улучшенная типографика и читаемость:** подключены системные шрифты Windows (`Segoe UI`, `Consolas`, `Segoe UI Emoji`), увеличен размер шрифта и высота строк таблицы, настроен высокий контраст текста для светлой и тёмной темы.
* **Обновлённый дизайн главного окна:** блоки правил, навигации и предпросмотра оформлены в виде скруглённых карточек с цветовыми акцентами заголовков.
* **Окно «ℹ О программе»:** добавлено модальное окно с информацией о текущей версии сборки, списке возможностей, лицензии и кнопкой быстрого копирования версии в буфер обмена.

### 🇬🇧 What's New
* **Enhanced Typography & Readability:** integrated native Windows fonts (`Segoe UI`, `Consolas`, `Segoe UI Emoji`), increased base font size and table row height, and improved text contrast across Light and Dark themes.
* **Refined Main Window UI:** redesigned rule blocks, folder explorer, and item preview into rounded card containers with accented section headers.
* **About Dialog (`ℹ About`):** added an information modal displaying the current build version, feature summary, license details, and a one-click version copy button.

---

## [v1.1.0] — 2026-11
### 🇷🇺 Что нового
* **Предпросмотр объектов (Item Preview):** встроенный просмотр миниатюр и разрешения изображений (`jpg`, `png`, `webp`, `bmp`, `ico`, `gif`), а также чтение первых 4 КБ текстовых файлов и исходного кода.
* **Удаление в Корзину (Recycle Bin):** безопасное пакетное и одиночное удаление файлов и папок в стандартную Корзину Windows с окном подтверждения.
* **Сортировка и фильтрация:** натуральная сортировка (`1, 2, 10`), сортировка по клику на заголовки столбцов, фильтрация по размеру, стоп-словам, статусу и защита системных папок (`.git`, `node_modules`).
* **Транслитерация и расширения:** перевод кириллицы в латиницу (`RU → LAT`) и пакетная смена регистра или типа расширения файлов.

### 🇬🇧 What's New
* **Item Preview:** built-in thumbnail and resolution preview for images (`jpg`, `png`, `webp`, `bmp`, `ico`, `gif`) and first 4 KB text/code preview.
* **Recycle Bin Deletion:** safe batch and single-item deletion to the Windows Recycle Bin with confirmation dialog.
* **Sorting & Filtering:** natural sorting (`1, 2, 10`), clickable column headers, file size and stop-word filters, status filter, and hidden/system folder protection (`.git`, `node_modules`).
* **Transliteration & Extensions:** Cyrillic-to-Latin conversion (`RU → LAT`) and batch file extension transformation.

---

## [v1.0.0] — 2026-11
* Первый публичный релиз **ReStem** (рекурсивное переименование файлов и папок, регулярные выражения Regex, автонумерация, Drag-and-Drop, переключение языков `RU / EN`, светлая и тёмная темы, откат `Undo`).
* Initial public release of **ReStem** (recursive file and folder renaming, Regex support, sequential numbering, Drag-and-Drop, `RU / EN` localization, Light and Dark themes, `Undo` support).
