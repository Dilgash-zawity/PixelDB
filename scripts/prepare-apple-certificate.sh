#!/usr/bin/env bash
# Turns the APPLE_CERTIFICATE secret into the base64 .p12 that Tauri's
# `security import` accepts, or fails with the reason it cannot.
#
# The secret may be a base64 .p12 (line-wrapped or not), a base64 PEM bundle
# or a raw PEM bundle with the certificate and its private key. macOS'
# keychain cannot read .p12 files made by OpenSSL 3's defaults (AES-256), so
# the identity is always re-exported with SHA1/3DES, which it can.
#
# Needs APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD and APPLE_SIGNING_IDENTITY;
# prints the new base64 .p12 on stdout.
set -euo pipefail

fail() { echo "::error title=Apple certificate::$1" >&2; exit 1; }

if [ -x "$(command -v brew)" ] && [ -x "$(brew --prefix openssl@3 2>/dev/null)/bin/openssl" ]; then
  openssl="$(brew --prefix openssl@3)/bin/openssl"
else
  openssl=openssl
fi
"$openssl" version | grep -q '^OpenSSL 3' || fail "OpenSSL 3 is required, found: $("$openssl" version)"

dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
export APPLE_CERTIFICATE_PASSWORD

printf '%s' "$APPLE_CERTIFICATE" > "$dir/secret"
if grep -q -- '-----BEGIN' "$dir/secret"; then
  cp "$dir/secret" "$dir/bundle.pem"
else
  tr -d ' \t\r\n' < "$dir/secret" | "$openssl" base64 -d -A > "$dir/decoded" 2>/dev/null ||
    fail 'APPLE_CERTIFICATE is not base64. Set it to the output of: base64 -i certificate.p12 | tr -d "\n"'
  [ -s "$dir/decoded" ] || fail 'APPLE_CERTIFICATE is not base64. Set it to the output of: base64 -i certificate.p12 | tr -d "\n"'
  if grep -q -- '-----BEGIN' "$dir/decoded"; then
    cp "$dir/decoded" "$dir/bundle.pem"
  elif "$openssl" pkcs12 -in "$dir/decoded" -passin env:APPLE_CERTIFICATE_PASSWORD -nodes -out "$dir/bundle.pem" 2> "$dir/err" ||
       "$openssl" pkcs12 -legacy -in "$dir/decoded" -passin env:APPLE_CERTIFICATE_PASSWORD -nodes -out "$dir/bundle.pem" 2>> "$dir/err"; then
    :
  elif grep -qi 'mac verify\|invalid password' "$dir/err"; then
    fail 'APPLE_CERTIFICATE_PASSWORD is not the password the .p12 was exported with.'
  elif "$openssl" x509 -inform DER -in "$dir/decoded" -noout 2> /dev/null; then
    fail 'APPLE_CERTIFICATE is a certificate (.cer) without its private key. In Keychain Access, expand the "Developer ID Application" certificate, select it with its private key and export both as a .p12.'
  else
    fail "APPLE_CERTIFICATE does not decode to a .p12 or PEM file: $(head -n 1 "$dir/err")"
  fi
fi

# The private key (decrypted if the PEM bundle had it encrypted).
"$openssl" pkey -in "$dir/bundle.pem" -passin env:APPLE_CERTIFICATE_PASSWORD -out "$dir/key.pem" 2> /dev/null ||
  fail 'APPLE_CERTIFICATE has no private key (or its key does not open with APPLE_CERTIFICATE_PASSWORD). Export the certificate together with its private key as a .p12.'

# Every certificate in the bundle; the one whose common name is the signing
# identity becomes the identity, the others travel along as its chain.
awk -v dir="$dir" '/-----BEGIN CERTIFICATE-----/ { n++; out = dir "/cert" n ".pem" } out { print > out } /-----END CERTIFICATE-----/ { close(out); out = "" }' "$dir/bundle.pem"
: > "$dir/chain.pem"
found=''
names=''
for cert in "$dir"/cert*.pem; do
  [ -e "$cert" ] || fail 'APPLE_CERTIFICATE has no certificate in it.'
  name=$("$openssl" x509 -in "$cert" -noout -subject -nameopt multiline,utf8,-esc_msb | sed -n 's/^ *commonName *= //p')
  names="$names${names:+, }\"$name\""
  if [ -z "$found" ] && [ "$name" = "$APPLE_SIGNING_IDENTITY" ]; then
    found=$cert
  else
    cat "$cert" >> "$dir/chain.pem"
  fi
done
[ -n "$found" ] || fail "APPLE_SIGNING_IDENTITY must be the certificate's full name. The certificate is $names."
[ "$("$openssl" x509 -in "$found" -noout -pubkey)" = "$("$openssl" pkey -in "$dir/key.pem" -pubout)" ] ||
  fail 'The private key in APPLE_CERTIFICATE belongs to a different certificate.'
"$openssl" x509 -in "$found" -noout -checkend 0 > /dev/null || fail "The certificate \"$APPLE_SIGNING_IDENTITY\" has expired."

chain=()
[ -s "$dir/chain.pem" ] && chain=(-certfile "$dir/chain.pem")
"$openssl" pkcs12 -export -inkey "$dir/key.pem" -in "$found" "${chain[@]}" \
  -certpbe PBE-SHA1-3DES -keypbe PBE-SHA1-3DES -macalg sha1 \
  -passout env:APPLE_CERTIFICATE_PASSWORD -out "$dir/identity.p12"

# The same import Tauri runs, in a throwaway keychain.
if command -v security > /dev/null; then
  security create-keychain -p check "$dir/check.keychain-db"
  security import "$dir/identity.p12" -k "$dir/check.keychain-db" -f pkcs12 -P "$APPLE_CERTIFICATE_PASSWORD" > /dev/null 2> "$dir/err" ||
    { security delete-keychain "$dir/check.keychain-db"; fail "The keychain still cannot import the certificate: $(cat "$dir/err")"; }
  security delete-keychain "$dir/check.keychain-db"
fi

"$openssl" base64 -A -in "$dir/identity.p12"
