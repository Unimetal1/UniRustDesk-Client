# Klient 1.3.6 z uwierzytelnianiem CNG

Baza klienta, interfejsu i lokalnej biblioteki `libs/hbb_common`:
`4b066b1fbaa8d5d6f9b53cb1e5b25e484f229da1`, sprzed wydzielenia biblioteki.
Historyczna licencja jest w `LICENCE`; jej kopia i opis pochodzenia znajdują się
również w `libs/hbb_common`.

Przeniesione modyfikacje:

- Wbudowany adres serwera i jego publiczny klucz przez `CNG_SERVER_ADDRESS`
  oraz `CNG_SERVER_PUBLIC_KEY`.
- Podpis wyzwania certyfikatem `CN=RustDesk` z magazynu LocalMachine/My.
  Klucz prywatny pozostaje w Windows CNG.
- Uwierzytelnione połączenia technika z hbbs i hbbr na portach 21118 i 21119.
- Niezależne warstwy szyfrowania transportu i sesji, z zachowaniem liczników.
- Rejestracja odbiorcy przez UDP, lokalne ustawienia serwera widoczne w GUI,
  wyłączenie wyprowadzania adresu opcjonalnego API w kompilacji CNG.
- Komunikaty certyfikatów po polsku i angielsku oraz poprawka DXGI → GDI.

WebSocket/WSS i funkcje dodane do klienta po tej bazie nie są częścią tej wersji.
Starsza wtyczka obrazu wymaga wymuszenia nagłówka `chrono` przy kompilacji MSVC;
poprawka jest zapisana w `flutter/windows/CMakeLists.txt`.

## Odtworzenie kompilacji Windows

Wymagane: Visual Studio 2022 C++, Rust, vcpkg z bibliotekami projektu,
Flutter 3.24.5 i flutter_rust_bridge_codegen 1.80.1 z obsługą UUID.
Źródła hbb_common są w repozytorium; nie pobiera się ich jako submodułu.

Z katalogu klienta, w środowisku Developer PowerShell z ustawionym VCPKG_ROOT:

```powershell
Push-Location flutter
flutter pub get
Pop-Location
$env:RUST_LOG = 'info'
flutter_rust_bridge_codegen --rust-input ./src/flutter_ffi.rs --dart-output ./flutter/lib/generated_bridge.dart --c-output ./flutter/macos/Runner/bridge_generated.h
$env:CNG_SERVER_ADDRESS = '86.63.69.254'
$env:CNG_SERVER_PUBLIC_KEY = (Get-Content PATH_TO_SERVER_PUBLIC_KEY -Raw).Trim()
cargo build --locked --features flutter --lib --release
Push-Location flutter
flutter build windows --release
Pop-Location
$Release = (Resolve-Path flutter/build/windows/x64/runner/Release).Path
Copy-Item target/release/deps/dylib_virtual_display.dll $Release -Force
```

`86.63.69.254` to domyślny serwer wbudowany w udostępniany klient RustDesk CNG.
Aby zbudować klienta dla innego serwera, podaj jego adres i klucz publiczny.

Wynikiem jest cały katalog `flutter/build/windows/x64/runner/Release`.
Program wymaga dołączonych DLL i katalogu `data`; sam plik EXE nie wystarcza.

## Wersja portable (jeden plik EXE)

Udostępniany plik `RustDesk-CNG-1.3.6.exe` to katalog Release spakowany
generatorem projektu. Zamknij uruchomionego klienta i z katalogu klienta wykonaj:

```powershell
Push-Location libs/portable
py -3.11 -m pip install -r requirements.txt
py -3.11 ./generate.py -f "$Release" -o . -e "$Release\rustdesk.exe"
Pop-Location
Copy-Item target/release/rustdesk-portable-packer.exe RustDesk-CNG-1.3.6.exe -Force
Get-FileHash RustDesk-CNG-1.3.6.exe -Algorithm SHA256
```

Generator kompresuje zawartość katalogu Release do `libs/portable/data.bin`
i kompiluje program samorozpakowujący `rustdesk-portable-packer`.
Pliki `data.bin` i `app_metadata.toml` są generowane i pomijane przez Git.

## Weryfikacja

Przeszły sprawdzenie Rust oraz kompilacje biblioteki release i interfejsu Windows.
Testy szyfrowania są w `libs/hbb_common/src/tcp.rs`.
Test `examples/cng_compatibility.rs` korzysta z rzeczywistych źródeł
uwierzytelniania klienta i jego funkcji wymiany klucza; podpis wykonuje Windows
CNG. Skrypt `tests/cng_compatibility.py` uruchamia tymczasowe serwery lokalne.

Test wymaga wolnych portów 21116–21119, lokalnego adresu IPv4 spoza loopback,
ważnego certyfikatu RustDesk z dostępnym kluczem prywatnym, publicznego pliku
tego certyfikatu, PyNaCl i binariów debug serwera w sąsiednim rustdesk-server.
Nie eksportuje klucza prywatnego ani nie kontaktuje się z serwerem produkcyjnym.

```powershell
Remove-Item Env:CNG_SERVER_ADDRESS,Env:CNG_SERVER_PUBLIC_KEY -ErrorAction SilentlyContinue
cargo build --locked --features flutter --example cng_compatibility
python tests/cng_compatibility.py LOCAL_IPV4 PUBLIC_CERTIFICATE.cer
```

Test protokołu nie zastępuje próby obrazu, sterowania i operacji na plikach
w pełnej sesji GUI pomiędzy dwoma komputerami.

W lokalnym teście z 16.09.2026 przeszły: podpis CNG i zaszyfrowane zapytanie ID,
rejestracja klucza i adresu odbiorcy przez UDP, przekazanie żądania relay przez
hbbs oraz dwukierunkowy transfer 64 B, 4 KiB i 1 MiB z jednoczesnym szyfrowaniem
sesji i transportu. Serwery testowe zostały zakończone po teście.

## Kopia stanu przed migracją

`../.build-logs/client-136-backup` zawiera archiwum plików, kopię indeksu Git,
patche i zapis wcześniejszego HEAD. Katalog `libs/hbb_common - kopia` nie został
zmieniony i jest pomijany przez Git. Migracja nie tworzy commita ani nie zmienia
historii gałęzi.
