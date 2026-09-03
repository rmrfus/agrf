# Installing agrf on a box without nix.
#
# `cargo install` copies binaries and nothing else — no man pages — and the
# release tarballs carry the bare musl binary, so the man page has no other way
# onto a machine. Everything here honours the usual PREFIX and DESTDIR, which
# is the language an AUR or Debian packager already speaks.
#
#   make && sudo make install                 # /usr/local
#   make && make install PREFIX="$HOME/.local"
#   make install DESTDIR="$pkgdir" PREFIX=/usr

PREFIX  ?= /usr/local
BINDIR  ?= $(PREFIX)/bin
MANDIR  ?= $(PREFIX)/share/man

CARGO   ?= cargo
INSTALL ?= install

BIN := target/release/agrf

.PHONY: all build install uninstall clean

all: build

build:
	$(CARGO) build --release --locked

# Deliberately not dependent on `build`: this is the target run under sudo,
# and rebuilding as root leaves target/ owned by root for the rest of time.
install:
	@test -x '$(BIN)' || { echo 'agrf: $(BIN) is missing — run `make` first' >&2; exit 1; }
	$(INSTALL) -Dm755 $(BIN)          $(DESTDIR)$(BINDIR)/agrf
	$(INSTALL) -Dm644 man/man1/agrf.1 $(DESTDIR)$(MANDIR)/man1/agrf.1

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/agrf
	rm -f $(DESTDIR)$(MANDIR)/man1/agrf.1

clean:
	$(CARGO) clean
