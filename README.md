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

**ReStem** — это легковесная нативная десктопная утилита для пакетного переименования файлов и каталогов, написанная на языке **Rust** с использованием графического движка `eframe` (`egui`). Программа компилируется в единый портативный `.exe` файл размером ~3–4 МБ, запускается за миллисекунды, не требует установки сторонних библиотек и плавно отображает десятки тысяч объектов благодаря виртуальной таблице.

### ✨ Ключевые возможности

* **Работа с файлами, папками и вложенными подкаталогами:**
  * Гибкая фильтрация объектов: переименовывайте только файлы, только папки или всё дерево каталогов рекурсивно.
  * **Безопасный иерархический обход (Bottom-Up):** при переименовании вложенных структур программа автоматически начинает с самого глубокого уровня вложенности и движется к корню, исключая ошибки потери пути.
  * У папок с точками в названии (например, `Отчет_2026.11`) часть после точки не воспринимается как расширение файла — имя каталога обрабатывается целиком.
* **Предпросмотр в реальном времени и точечный выбор:**
  * Любое изменение правил мгновенно отображается в колонке **«Новое имя»** с контрастной цветовой подсветкой.
  * Индивидуальные галочки (чекбоксы) напротив каждой строки позволяют исключить отдельные файлы или папки из пакетной обработки, а кнопка **«Инвертировать»** и главный чекбокс в шапке таблицы ускоряют массовый выбор.
  * Сквозной счётчик нумерации автоматически пропускает снятые с выбора строки, не создавая пропусков в последовательности номеров.
* **Трёхколоночный интерфейс в стиле современного Проводника:**
  * **Левая панель:** встроенная навигация по подпапкам (`⬆ На уровень вверх` и быстрый переход в дочерние каталоги) + блок **«Шаблоны правил»** для применения популярных настроек в 1 клик.
  * **Центральная панель:** виртуальная таблица с отображением типа объекта (`📁 Папка` / `📄 Файл`), относительного пути, предпросмотра нового имени, статуса и размера файла.
  * **Правая панель:** сгруппированный инспектор правил переименования.
* **Поддержка Drag-and-Drop:** перетащите любую папку или файл из Проводника Windows прямо в окно ReStem для мгновенной загрузки списка.
* **Двуязычный интерфейс и темы оформления:**
  * Мгновенное переключение языка интерфейса **RU / EN** на верхней панели.
  * Переключение между **☀ Светлой** и **🌙 Тёмной** темой в один клик.
* **Безопасность и откат (Undo):**
  * Автоматическая проверка конфликтов и дубликатов имён внутри каждой папки до начала переименования.
  * Кнопка **«⮌ Откат (Undo)»** возвращает прежние имена всем переименованным файлам и папкам в строго обратном порядке (сверху вниз).

---

### 🛠 Правила и инструменты переименования

1. **Поиск и замена (Find & Replace):**
   * Замена текста с поддержкой **регулярных выражений (Regex)** и группами захвата (`$1`, `$2`).
   * Опция учёта регистра символов.
2. **Трансформация регистра (Case Transformation):**
   * `Не менять` / `все строчные` / `ВСЕ ПРОПИСНЫЕ` / `Первая буква заглавная` / `Каждое Слово С Заглавной`.
   * Полная поддержка кириллицы и символов UTF-8.
3. **Префикс, суффикс и обрезка (Add & Trim):**
   * Добавление произвольного текста в начало имени или перед расширением файла.
   * Посимвольная обрезка заданного количества знаков слева и справа.
4. **Автонумерация (Serialization):**
   * Добавление номера в начало (префикс) или в конец имени (суффикс).
   * Настройка стартового числа, шага, разделителя и количества ведущих нулей (`01`, `001`, `0001`).
   * Опция **«Сброс счётчика в подпапках»** для независимой нумерации внутри каждого вложенного каталога.
5. **Фильтрация и поиск:**
   * Быстрый поиск по подстроке в верхней панели инструментов.
   * Фильтр по списку расширений (например: `jpg, png, pdf`).

---

### 🚀 Скачивание и запуск

Готовый исполняемый файл не требует установки Python, Rust или дополнительных библиотек:
1. Перейдите в раздел [**Releases**](../../releases) (или скачайте артефакт `ReStem-Portable` во вкладке **Actions**).
2. Запустите `restem.exe`.
3. Нажмите **«📁 Открыть папку...»** или перетащите нужный каталог мышью в окно программы.

---
---

## 🇬🇧 ReStem — Fast Bulk File & Folder Renamer

**ReStem** is a lightweight, native desktop utility for batch renaming files and directories, built in **Rust** using the `eframe` (`egui`) immediate-mode GUI framework. It compiles into a single portable `.exe` (~3–4 MB) with zero external dependencies, launches instantaneously, and smoothly handles tens of thousands of items using virtual table rendering.

### ✨ Key Features

* **Files, Folders & Recursive Subdirectories:**
  * Choose whether to rename files, folders, or entire directory trees recursively.
  * **Safe Bottom-Up Traversal:** when renaming nested directories and files simultaneously, ReStem automatically sorts operations from the deepest nesting level up to the root so parent folder renames never break child paths.
  * Folder names containing dots (e.g., `Report_2026.11`) are treated as complete names rather than split into file extensions.
* **Real-Time Preview & Granular Selection:**
  * Every rule adjustment updates the **New Name** column in real time with pill-style color highlighting.
  * Individual checkboxes next to every row let you exclude specific items, while the master header checkbox and **Invert** button speed up batch selection.
  * Sequential numbering automatically skips unchecked rows so your numbering sequence stays contiguous without gaps.
* **Modern 3-Column Explorer Layout:**
  * **Left Panel:** built-in folder navigation (`⬆ Up one level` and quick subfolder switching) + **Saved Presets** for applying common renaming patterns in a single click.
  * **Center Panel:** high-performance virtual table displaying item type (`📁 Folder` / `📄 File`), relative path, live new name preview, status, and file size.
  * **Right Panel:** structured Rename Rules inspector.
* **Drag-and-Drop Support:** drag any folder or file directly from Windows Explorer into the ReStem window to load it immediately.
* **Bilingual UI & Theme Switcher:**
  * Instant on-the-fly **RU / EN** language switching in the top toolbar.
  * One-click toggle between **☀ Light** and **🌙 Dark** themes.
* **Safety Checks & Full Undo:**
  * Pre-flight validation prevents duplicate name collisions within the same directory before touching the disk.
  * The **⮌ Undo** button restores original file and folder names in reverse hierarchical order (top-down).

---

### 🛠 Renaming Rules & Capabilities

1. **Find & Replace:**
   * Plain text or full **Regular Expression (Regex)** matching with capture group support (`$1`, `$2`).
   * Optional case sensitivity toggle.
2. **Case Transformation:**
   * `Default (No change)` / `lowercase` / `UPPERCASE` / `Capitalize first letter` / `Title Case Words` with full Unicode/UTF-8 support.
3. **Add Prefix/Suffix & Trim:**
   * Insert custom text at the start of the stem or right before the extension.
   * Trim a specified number of characters from the left or right edge of the name.
4. **Serialization (Numbering):**
   * Prepend or append sequential numbers with customizable start value, step increment, separator, and zero-padding (`01`, `001`, `0001`).
   * **Reset counter per folder** option for independent numbering inside each subdirectory.
5. **Filtering & Search:**
   * Instant filename search bar in the top toolbar.
   * Extension whitelist filter (e.g., `jpg, png, pdf`).

---

### 🚀 Download & Usage

1. Go to the [**Releases**](../../releases) section (or download the `ReStem-Portable` artifact from the **Actions** tab).
2. Run `restem.exe` — no installation or runtime required.
3. Click **📁 Add Folder...** or drag and drop a folder into the application window.

---
