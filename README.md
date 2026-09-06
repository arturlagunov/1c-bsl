# 1c-bsl — Zed extension using official BSL Language Server

Zed-расширение, подключающее **официальный bsl-language-server** (Java, от 1c-syntax) к файлам `.bsl` в Zed.
Работает через полноценный **WASM-компонент** (не Rust-порт bsl-analyzer), собранный под WebAssembly Component Model.

## Что даёт

- Синтаксическая подсветка BSL / OneScript (`.bsl`, `.osl`) и SDBL
- LSP-функции от официального **BSL Language Server**:
  - переход к определению (`textDocument/definition`)
  - hover, автодополнение, синонимы
  - диагностика (подсветка ошибок/предупреждений)
  - code actions, rename, document symbols и др.
- Никакого скачивания jar при старте — путь к серверу задан явно

## Структура

```
1c-bsl/
├── extension.toml        # Manifest расширения Zed (языки + LSP-адаптер bsl)
├── extension.wasm        # Собранный WASM-компонент (ядро: команда запуска LSP)
├── src/lib.rs            # Исходник WASM-компонента
├── Cargo.toml
├── grammars/             # tree-sitter грамматики BSL / SDBL (.wasm)
├── languages/            # Конфигурации языков (BSL, SDBL, SDBL Embedded)
├── snippets/             # Сниппеты BSL
├── build.sh              # Сборка WASM-компонента
├── install.sh            # Установка в Zed
└── docs/                 # Документация
```

## Требования

- **Java 17+ (рекомендуется 21)** в PATH — Zed и расширение используют `java` из PATH.
  Минимальная Java зависит от версии bsl-language-server:
  - v1.0.x и новее — **Java 21**
  - до v0.28.x — Java 17
- **Zed** (Linux/macOS)

## Установка

```bash
bash install.sh
```

Скрипт:
1. Скачивает bsl-language-server.jar в `~/.local/lib/bsl-language-server/`
2. Копирует расширение в `~/.local/share/zed/extensions/installed/1c-bsl/`
3. Проверяет Java

### Настройка в settings.json (`~/.config/zed/settings.json`)

```jsonc
{
  "lsp": {
    "bsl": {
      "binary": {
        "path": "java",
        "arguments": ["-Xmx4g", "-jar", "/home/USER/.local/lib/bsl-language-server/bsl-language-server.jar"]
      }
    }
  },
  "languages": {
    "BSL": {
      "language_servers": ["bsl"],
      "format_on_save": "off"
    }
  }
}
```

> Уберите `lsp.bsl.binary` если команду даёт WASM-компонент (см. ниже). Обе команды должны указывать на один и тот же jar.

После установки — **полностью перезапустите Zed** (закрыть и открыть), и откройте `.bsl` файл.

## Сборка WASM-компонента из исходников

```bash
bash build.sh   # требует rustup + target wasm32-wasip2
```

Результат — `extension.wasm` (WebAssembly Component, не plain-module; plain-wasm Zed отвергает: `failed to compile wasm component`).

## Как это работает

Два независимых пути подключают сервер:

1. **WASM-компонент** (`src/lib.rs`) — реализует `language_server_command()` интерфейс
   Zed-расширения и возвращает команду `java -Xmx4g -jar <путь>`.
2. **settings.json** — `lsp.bsl.binary` дублирует ту же команду как запасной путь.

Имя адаптера `bsl` жёстко фиксировано manifest'ом расширения
(`[language_servers.bsl]` + `wasm = true`). Без `extension.wasm` адаптер не регистрируется —
Zed не позволяет создавать LSP-адаптеры только через settings.json (валидация имён).

## Реаныши и отладка

Журнал Zed: `~/.local/share/zed/logs/Zed.log`
- ищите `starting language server process. binary path: "java"`
- ошибки загрузки расширения: `Failed to load extension: 1c-bsl`

Уровень памяти (`-Xmx`):
- 1 GB достаточно для небольших проектов
- 4 GB рекомендуется для крупных конфигураций (БСП, ERP) — при нехватке heap JVM
  молча завершается без hs_err-файла, подключение обрывается (`server shut down`)

## Лицензия

Производная работа от расширения [Nesterland/zed-1c-bsl](https://github.com/Nesterland/zed-1c-bsl) (MIT).
Ядро — [1c-syntax/bsl-language-server](https://github.com/1c-syntax/bsl-language-server).