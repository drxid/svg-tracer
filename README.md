# SVG Tracer

Локальное приложение для Windows: перетащил картинку, получил чистый SVG, скопировал в код. Объединяет VTracer, Potrace и SVGO в одном окне, изображения никуда не отправляются.

| Режим | Движок | Для чего |
|---|---|---|
| Цвет | VTracer | Иллюстрации и многоцветные изображения |
| Контур | Potrace | Монохромные иконки, силуэты, сканы |
| Логотип | Порт `trace.py` и Potrace | Плоская графика: без бахромы и зазоров между цветами |

Установщик — в [релизах](https://github.com/drxid/svg-tracer/releases/latest). Дальше ставить вручную ничего не надо: Potrace скачивается при первом запуске и проверяется по SHA-256, а новые версии приложение находит само и предлагает перезапуск кнопкой в шапке.

## Разработка

Нужны Rust 1.90, Node 22, pnpm 10 и MSVC Build Tools.

```bat
pnpm install
pnpm tauri dev      :: запуск
pnpm tauri build    :: установщик, нужен ключ подписи (см. ниже)
pnpm typecheck
pnpm test
cd src-tauri && cargo test --lib
```

## Релиз

```bat
pnpm version patch
git push --follow-tags
```

Команда поднимает версию в `package.json`, создаёт коммит и тег, а по тегу GitHub Actions прогоняет тесты, собирает установщик и публикует релиз с `latest.json`.

Обновления подписаны. Закрытый ключ лежит в секрете `TAURI_SIGNING_PRIVATE_KEY`, открытый — в `tauri.conf.json`. Для локальной сборки задайте ключ в переменной окружения с тем же именем и `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` с пустым значением.

## Сверка с эталоном

Порт логотипов проверяется по исходному `legacy/trace.py`: SSIM ≥ 0,98 и разница числа узлов ≤ 15 %. Нужен `.venv` с зависимостями из `legacy/requirements.txt` и собранный пример:

```bat
cd src-tauri && cargo build --release --example trace_file && cd ..
.venv\Scripts\python scripts\make_fixtures.py
.venv\Scripts\python scripts\compare_with_trace_py.py "%LOCALAPPDATA%\svg-tracer\modules\potrace\1.16\potrace.exe"
```

## Лицензия

Код приложения распространяется по лицензии [MIT](LICENSE).

[Potrace](https://potrace.sourceforge.net/) 1.16 распространяется по лицензии GPL-2.0. Он не входит в репозиторий и установщик: приложение скачивает с сайта автора неизменённую программу и запускает её как отдельный процесс.
