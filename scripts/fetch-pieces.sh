#!/usr/bin/env sh
set -eu
mkdir -p assets/pieces
base="https://raw.githubusercontent.com/bend-n/chess/da9b965e1858852cc26b22fa670c9e22b338b1f4/assets/pieces/cburnett"
for piece in wK wQ wR wB wN wP bK bQ bR bB bN bP; do
  curl -fsSL "$base/$piece.png" -o "assets/pieces/$piece.png"
done
