<div align="center">

# ReStem

**Быстрое пакетное переименование файлов и каталогов на Rust**  
**Fast Bulk File & Folder Renamer written in Rust**

[![Language: Rust](https://img.shields.io/badge/Rust-2021_Edition-orange?logo=rust)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-eframe_%2F_egui-00b4d8)](https://github.com/emilk/egui)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows_x64-0078d4?logo=windows)](https://github.com/)
[![License: MIT](https://img.shields.io/badge/License-MIT-success)](LICENSE)

[**🇷🇺 Русская документация**](#-restem--пакетное-переименование-файлов-и-папок) &nbsp;|&nbsp; [**🇬🇧 English Documentation**](#-restem--fast-bulk-file--folder-renamer)

</div>

---

## 🇷🇺 ReStem — Пакетное переименование файлов и папок

**ReStem** — это легковесная нативная десктопная утилита для пакетного переименования файлов и каталогов, написанная на языке **Rust** с использованием графического движка `eframe` (`egui`). Программа работает из единого портативного `.exe` файла размером ~3–4 МБ, запускается за миллисекунды, не требует установки сторонних библиотек и плавно отображает десятки тысяч объектов благодаря виртуальной таблице.

### ✨ Ключевые возможности

* **Работа с файлами, папками и вложенными подкаталогами:**
  * Гибкая фильтрация объектов: переименовывайте только файлы, только папки или всё дерево каталогов рекурсивно.
  * **Безопасный иерархический обход (Bottom-Up):** при переименовании вложенных структур программа автоматически начинает с самого глубокого уровня вложенности и движется к корню, исключая ошибки потери пути.
  * У папок с точками в названии (например, `Отчет_2026.11`) часть после точки не воспринимается как расширение файла — имя каталога обрабатывается целиком.
* **Предпросмотр в реальном времени и точечный выбор:**
  * Любое изменение правил мгновенно отображается в колонке **«Новое имя»** с контрастной цветовой подсветкой.
  * Индивидуальные галочки (чекбоксы) напротив каждой строки позволяют исключить отдельные файлы или папки из пакетной обработки, а кнопка **«Инвертировать»** и главный чекбокс в шапке таблицы ускоряют массовый выбор.
  * Сквозной счётчик нумерации автоматически пропускает снятые с выбора строки, не создавая пропусков в последовательности номеров.
* **Гибкая сортировка списка:**
  * **Умная сортировка по имени (Natural Sort: `1, 2, 10`):** распознаёт числа внутри строк (`Кадр_2.jpg` идёт перед `Кадр_10.jpg`).
  * Сортировка по **алфавиту (`А–Я`)**, **дате изменения**, **размеру файла**, **расширению/типу** и **расположению (пути)**.
  * Быстрая сортировка в один клик по заголовкам столбцов таблицы (`Тип ↕`, `Текущее имя ↕`, `Размер ↕`, `Изменён ↕`), переключение направления (`⬆ По возр.` / `⬇ По убыв.`) и опция фиксации **«Папки сверху»**.
* **Многоуровневая фильтрация и защита служебных папок:**
  * Фильтр отображения по статусу: *Все объекты*, *Только изменяемые*, *Только выбранные галочкой ☑*, *Без изменений*.
  * Поиск по имени в верхней панели с поддержкой **регулярных выражений (`.*`)** и учёта регистра (`Aa`).
  * Автоматическое скрытие системных и скрытых файлов Windows, а также служебных каталогов (`.git`, `node_modules`, `$RECYCLE.BIN`).
* **Трёхколоночный интерфейс в стиле современного Проводника:**
  * **Левая панель:** встроенная навигация по подпапкам (`⬆ На уровень вверх` и быстрый переход в дочерние каталоги) + блок **«Шаблоны правил»** для применения популярных настроек в 1 клик.
  * **Центральная панель:** виртуальная таблица с отображением типа объекта (`📁 Папка` / `📄 Файл`), относительного пути, предпросмотра нового имени, статуса, размера и даты модификации.
  * **Правая панель:** сгруппированный инспектор из 6 модулей правил и фильтров.
* **Поддержка Drag-and-Drop:** перетащите любую папку или файл из Проводника Windows прямо в окно ReStem для мгновенной загрузки списка.
* **Двуязычный интерфейс и темы оформления:**
  * Мгновенное переключение языка интерфейса **RU / EN** на верхней панели.
  * Переключение между **☀ Светлой** и **🌙 Тёмной** темой в один клик.
* **Безопасность и откат (Undo):**
  * Автоматическая проверка конфликтов и дубликатов имён внутри каждой папки до начала переименования.
  * Кнопка **«⮌ Откат (Undo)»** возвращает прежние имена всем переименованным файлам и папкам в строго обратном порядке (сверху вниз).

---

### 🛠 Правила переименования и фильтры (Правая панель)

1. **Поиск и замена (Find & Replace):**
   * Замена текста с поддержкой **регулярных выражений (Regex)** и группами захвата (`$1`, `$2`).
   * Опция учёта регистра символов.
2. **Регистр и транслитерация (Case & Transliteration):**
   * Режимы регистра: `Не менять регистр` / `все строчные` / `ВСЕ ПРОПИСНЫЕ` / `Первая буква заглавная` / `Каждое Слово С Заглавной` с полной поддержкой кириллицы и UTF-8.
   * **Транслитерация кириллицы (`RU → LAT`):** автоматический перевод русских букв в латиницу (`ж → zh`, `ч → ch`, `ш → sh`, `щ → shch`, `ю → yu`, `я → ya`, `ъ/ь` опускаются) с сохранением регистра и возможностью одновременного приведения к строчным или прописным буквам.
3. **Префикс, суффикс и обрезка (Add & Trim):**
   * Добавление произвольного текста в начало имени или перед расширением файла.
   * Посимвольная обрезка заданного количества знаков слева и справа.
4. **Автонумерация (Numbering):**
   * Добавление номера в начало (префикс) или в конец имени (суффикс) **строго в порядке текущей сортировки таблицы** (например, можно отсортировать файлы по дате изменения и пронумеровать их хронологически).
   * Настройка стартового числа, шага, разделителя и количества ведущих нулей (`01`, `001`, `0001`).
   * Опция **«Сброс счётчика в подпапках»** для независимой нумерации внутри каждого вложенного каталога.
5. **Расширения файлов (File Extensions):**
   * Перевод расширений в `строчные (.jpg)`, `ПРОПИСНЫЕ (.JPG)` или полное удаление расширения.
   * Пакетная замена текущего расширения на новое (например, `jpeg` → `jpg`). На имена папок правила расширений не влияют.
6. **Фильтры и исключения (Advanced Filters):**
   * **Белый список расширений:** работа только с указанными форматами (`jpg, png, pdf`).
   * **Чёрный список расширений:** исключение ненужных форматов (`tmp, bak, ini`).
   * **Исключение по словам:** скрытие объектов, содержащих заданные слова или фрагменты через запятую.
   * **Фильтр по размеру файла:** выбор файлов *Больше чем N КБ*, *Меньше чем N КБ* или в *Диапазоне от N до M КБ*.

---

### 🚀 Скачивание и запуск

Готовый исполняемый файл не требует установки дополнительных библиотек:
1. Перейдите в раздел [**Releases**](../../releases) и скачайте `restem.exe`.
2. Запустите `restem.exe`.
3. Нажмите **«📁 Открыть папку...»** или перетащите нужный каталог мышью в окно программы.

---

### 📄 Лицензия

Распространяется под лицензией **MIT License**. Свободно для личного и коммерческого использования.

---
---

## 🇬🇧 ReStem — Fast Bulk File & Folder Renamer

**ReStem** is a lightweight, native desktop utility for batch renaming files and directories, built in **Rust** using the `eframe` (`egui`) immediate-mode GUI framework. It runs from a single portable `.exe` (~3–4 MB) with zero external dependencies, launches instantaneously, and smoothly handles tens of thousands of items using virtual table rendering.

### ✨ Key Features

* **Files, Folders & Recursive Subdirectories:**
  * Choose whether to rename files, folders, or entire directory trees recursively.
  * **Safe Bottom-Up Traversal:** when renaming nested directories and files simultaneously, ReStem automatically sorts operations from the deepest nesting level up to the root so parent folder renames never break child paths.
  * Folder names containing dots (e.g., `Report_2026.11`) are treated as complete names rather than split into file extensions.
* **Real-Time Preview & Granular Selection:**
  * Every rule adjustment updates the **New Name** column in real time with pill-style color highlighting.
  * Individual checkboxes next to every row let you exclude specific items, while the master header checkbox and **Invert** button speed up batch selection.
  * Sequential numbering automatically skips unchecked rows so your numbering sequence stays contiguous without gaps.
* **Multi-Mode Sorting:**
  * **Natural Name Sort (`1, 2, 10`):** smart numerical sorting (`Frame_2.jpg` comes before `Frame_10.jpg`).
  * Sort by **Alphabetical (`A–Z`)**, **Modified Date**, **File Size**, **Extension / Type**, or **Folder Path**.
  * Clickable table column headers (`Type ↕`, `Original Name ↕`, `Size ↕`, `Modified ↕`), ascending/descending toggle (`⬆ Asc` / `⬇ Desc`), and **Folders first** pinning.
* **Advanced Filtering & System File Protection:**
  * Filter rows by status: *All items*, *Only changing*, *Only checked ☑*, or *Unchanged only*.
  * Top toolbar search bar with **Regular Expression (`.*`)** and **Case-Sensitive (`Aa`)** toggles.
  * Automatic filtering of hidden/system Windows files and service directories (`.git`, `node_modules`, `$RECYCLE.BIN`).
* **Modern 3-Column Explorer Layout:**
  * **Left Panel:** built-in folder navigation (`⬆ Up one level` and quick subfolder switching) + **Saved Presets** for applying common renaming patterns in a single click.
  * **Center Panel:** high-performance virtual table displaying item type (`📁 Folder` / `📄 File`), relative path, live new name preview, status, file size, and modification timestamp.
  * **Right Panel:** structured 6-section Rename Rules & Filters inspector.
* **Drag-and-Drop Support:** drag any folder or file directly from Windows Explorer into the ReStem window to load it immediately.
* **Bilingual UI & Theme Switcher:**
  * Instant on-the-fly **RU / EN** language switching in the top toolbar.
  * One-click toggle between **☀ Light** and **🌙 Dark** themes.
* **Safety Checks & Full Undo:**
  * Pre-flight validation prevents duplicate name collisions within the same directory before touching the disk.
  * The **⮌ Undo** button restores original file and folder names in reverse hierarchical order (top-down).

---

### 🛠 Renaming Rules & Filters (Right Panel)

1. **Find & Replace:**
   * Plain text or full **Regular Expression (Regex)** matching with capture group support (`$1`, `$2`).
   * Optional case sensitivity toggle.
2. **Case & Transliteration:**
   * Case modes: `Default (No change)` / `lowercase` / `UPPERCASE` / `Capitalize first letter` / `Title Case Words` with full Unicode/UTF-8 support.
   * **Cyrillic Transliteration (`RU →
