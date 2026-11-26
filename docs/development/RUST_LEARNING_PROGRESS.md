# Rust Learning Progress

> Фактический прогресс обучения Rust и контроля качества.
>
> Этот файл — **единственный источник истины** о состоянии обучения. Он имеет
> приоритет над историей переписки. При начале новой сессии агент обязан
> прочитать его до любого учебного действия.
>
> Правила заполнения, оценивания и перехода между темами описаны в
> [`RUST_LEARNING_TRACKING.md`](./RUST_LEARNING_TRACKING.md). Учебная программа —
> в [`RUST_LEARNING_PLAN.md`](./RUST_LEARNING_PLAN.md).

## Current Position

- Program: 12 weeks × 5 days = 60 lessons
- Current week: 1
- Current lesson: Week 1, Day 1
- Current topic: Основы Rust, Cargo и структура проекта
- Current task: создать `training/rust-basics`, исключить `training` из workspace, разобрать структуру Cargo-проекта
- Lesson status: `TECHNICAL_REWORK`
- Last accepted lesson: None
- Next allowed lesson: Week 1, Day 1 (текущее — пересдача технической части)
- Training started: yes (2026-11-26)

> Занятие Week 1, Day 1 выдано 2026-11-26 по команде пользователя.
> Technical Assessment проведён 2026-11-26: 43/100 — ниже порога 85.
> Learning Assessment **не проводился** (запрещено при непройденной технике).
> Переход к Day 2 запрещён до `ACCEPTED`.

## Overall Progress

- Total lessons: 60
- Accepted lessons: 0
- Technical rework: 1
- Learning rework: 0
- Blocked lessons: 0

## Active Assessment

- Technical Assessment: 43/100 — `TECHNICAL_REWORK` (порог 85)
- Learning Assessment: — (не проводилась: техника не принята)

Оценки выставляются только по факту выполненной проверки. Отсутствие значения
означает, что проверка ещё не запускалась, а не нулевой балл.

## Current Weak Topics

- Cargo-манифест: `edition` задаётся явно, а не оставляется по умолчанию.
- Нельзя выдавать версию одного инструмента (`rustup`) за версию другого (`rustc`).
- Артефакты сборки (`target/`, собранные бинарники) не попадают в Git.
- Задание требует конкретных значений — их нужно проверять, а не заполнять похожими.

## Topics To Repeat

- Структура `Cargo.toml`: `[package]`, `edition`, `[workspace]`, `exclude`.
- Отличие версии тулчейна, компилятора и пакета.

## Completed Lessons

| Week | Day | Topic | Technical | Learning | Status |
| ---- | --- | ----- | --------: | -------: | ------ |
| —    | —   | —     |         — |        — | —      |

## Assessment History

### 2026-11-26 — Week 1, Day 1 — попытка 1

**Статус:** `TECHNICAL_REWORK`

**Technical Assessment: 43/100** (порог 85)

| Критерий              | Баллы |
| --------------------- | ----: |
| Требования            |  7/25 |
| Корректность          | 10/20 |
| Архитектура           | 10/15 |
| Ошибки и безопасность |  5/10 |
| Качество кода         |  3/10 |
| Тесты                 |  3/15 |
| Регрессии             |  5/5  |

Веса не перераспределялись: все критерии применимы к заданию.

**Проверенные файлы:** `Cargo.toml` (корень), `training/rust-basics/Cargo.toml`,
`training/rust-basics/src/main.rs`, `training/rust-basics/Cargo.lock`,
`training/rust-basics/.gitignore`, `training/rust-basics/src/main`.

**Выполненные команды:**

| Команда | Результат |
| --- | --- |
| `cargo fmt --all -- --check` | PASS (workspace; `training/` в него не входит) |
| `rustfmt --check --edition 2024 src/main.rs` (в `training/rust-basics`) | FAIL — отступ табом |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace` | PASS, 115 тестов |
| `cargo run` в `training/rust-basics` | запускается, но с warning и неверным выводом |
| `cargo build` в `training/rust-basics` | warning: `package.edition` unspecified, defaulting to `2015` |
| `cargo metadata --no-deps` | `training` отсутствует в members — изоляция работает |
| `cargo pkgid -p rust-basics` (из корня) | «did not match any packages» — не в workspace |
| `git diff main...HEAD -- Cargo.lock` | пусто — lockfile продукта не изменён |

**Найденные проблемы:**

1. **В репозиторий закоммичен собранный бинарник** `training/rust-basics/src/main`
   (4 354 656 байт, ELF x86-64). Это артефакт сборки, его место в `target/`.
   Критическая ошибка.
2. **`edition` не задан**, хотя требование — `2024`. Cargo берёт `2015` и печатает
   предупреждение при каждой сборке. Требование задания не выполнено.
3. **Выводится версия не того инструмента.** Требовалось «версия Rust, с которой
   собрана программа». Выведено `rustup 1.29.3` — это менеджер тулчейнов, а не
   компилятор (`rustc 1.97.1`).
4. **Версия захардкожена строкой**, а не получена на этапе компиляции
   (`env!("CARGO_PKG_RUST_VERSION")` и т.п.). Программа не будет отражать факт
   сборки.
5. **Бинарник `rust-basics` удалён из workspace** через `[workspace]` внутри его
   же манифеста в дополнение к `exclude` в корне. Работает, но одно из двух
   решений лишнее и вводит в заблуждение.
6. **Лишний пробел в конце строки** `exclude = ["training"] ` в корневом
   `Cargo.toml`.
7. **Отступ табом** в `main.rs` — `rustfmt --check` падает.
8. **Коммит-сообщение с опечаткой** `created training/rust-busic` — каталог
   называется `rust-basics`.
9. **Ветка не отправлена в `origin`** — по §14.4 оценка выставляется по
   содержимому отправленной ветки; проверка выполнена на локальной ветке, это
   отмечено как отклонение от процесса.

**Что выполнено верно:** `exclude = ["training"]` добавлен в корневой манифест;
`Cargo.lock` продукта не изменился; `training/` действительно изолирован от
`--workspace`; программа запускается и печатает две строки; внешних зависимостей
нет; регрессий в workspace нет.

**План исправлений:**

1. `git rm --cached training/rust-basics/src/main`, добавить `/src/main` (или
   лучше собирать в `target/`) — бинарник не должен быть в Git.
2. Указать в `training/rust-basics/Cargo.toml`: `version = "0.1.0"`,
   `edition = "2024"`.
3. Убрать `[workspace]` из манифеста упражнения, оставить только `exclude` в
   корне.
4. Получить версию компилятора на этапе сборки, а не хардкодом.
5. Прогнать `cargo fmt` в `training/rust-basics`.
6. Убрать пробел в конце строки в корневом `Cargo.toml`.
7. Отправить ветку в `origin`.

**Learning Assessment: не проводился.** Согласно §6, при непройденной технической
части финальная учебная проверка не проводится.

## Project Tasks

Проектные задачи выполняются в рамках текущего этапа V0.1 и не должны выходить
за его границы (см. `AGENTS.md`, раздел 15).

| Date | Lesson | Task | Files | Status |
| ---- | ------ | ---- | ----- | ------ |
| —    | —      | —    | —     | —      |

## Blocked

- None.

## Lesson Branches

Каждое занятие — отдельная ветка от `main` с именем
`lesson/<week>-<day>-<topic>`, например `lesson/01-01-cargo-setup`
(см. `RUST_LEARNING_TRACKING.md`, раздел 14). Работа напрямую в `main` для
учебных задач запрещена.

| Lesson | Branch | Commit | Status | Merged |
| ------ | ------ | ------ | ------ | ------ |
| 01-01  | `lesson/01-01-cargo-setup` | `10d28dc` | TECHNICAL_REWORK | no |

## Session Log

| Date | Action | Result |
| ---- | ------ | ------ |
| 2026-11-25 | Система обучения внедрена | Обучение не начато, занятия не выдавались |
| 2026-11-25 | Репозиторий инициализирован, подключён `origin` | `main` отправлен в `zladen/trading-plaform`; CI зелёный |
| 2026-11-26 | Выдано занятие Week 1, Day 1 | Ветка `lesson/01-01-cargo-setup`, статус `IN_PROGRESS` |
| 2026-11-26 | Technical Assessment, попытка 1 | 43/100 — `TECHNICAL_REWORK`, Learning не проводился |

## Next Action

Дождаться исправлений по плану выше в ветке `lesson/01-01-cargo-setup`,
затем повторить Technical Assessment. Learning Assessment — только после
прохождения технической части. Week 1, Day 2 не начинать.
