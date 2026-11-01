#!/usr/bin/env python3
"""Reproduce public backup test fixtures with independent native implementations.

Optional Fedora tooling: python3, libargon2 and libsodium (runtime libraries).
Not a build/test dependency. Uses fixed PUBLIC test inputs, never user secrets.
Argon2 reference: https://github.com/P-H-C/phc-winner-argon2
XChaCha API: https://doc.libsodium.org/secret-key_cryptography/aead/chacha20-poly1305/xchacha20-poly1305_construction
STREAM nonce layout: https://docs.rs/aead-stream/0.6.0/src/aead_stream/lib.rs.html
"""

import ctypes
import ctypes.util


def library(name):
    path = ctypes.util.find_library(name)
    if path is None:
        raise SystemExit(f"Missing reference library: {name}")
    return ctypes.CDLL(path)


def main():
    argon = library("argon2")
    argon.argon2id_hash_raw.argtypes = [
        ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32,
        ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t,
        ctypes.c_void_p, ctypes.c_size_t,
    ]
    password = " 日本語🔑 e\u0301 ".encode("utf-8")
    salt = bytes([2] * 16)
    key = ctypes.create_string_buffer(32)
    if argon.argon2id_hash_raw(3, 65536, 4, password, len(password), salt, len(salt), key, 32) != 0:
        raise SystemExit("Reference Argon2 failed")
    expected = "cbae8d1f2bd793b9a25168e77dc1318d0e9f9f852575bf3b20ce9a604c996c4e"
    if key.raw.hex() != expected:
        raise SystemExit("Reference Argon2 fixture mismatch")
    print(f"Argon2 profile: {key.raw.hex()}")

    sodium = library("sodium")
    if sodium.sodium_init() < 0:
        raise SystemExit("Reference libsodium initialization failed")
    sodium.crypto_aead_xchacha20poly1305_ietf_encrypt.argtypes = [
        ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulonglong), ctypes.c_void_p,
        ctypes.c_ulonglong, ctypes.c_void_p, ctypes.c_ulonglong,
        ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
    ]
    fixtures = [
        (b"abc", b"first", False, "9affe98e47e430435f2c75eb7f95a2ddf1cd78"),
        (b"def", b"second", True, "503623bb3c0b9bc8dfb1f29bc8c758f1a4c8bc"),
    ]
    for counter, (plaintext, aad, last, expected) in enumerate(fixtures):
        # Reference-only construction; production delegates nonces to STREAM.
        nonce = bytes([0x22] * 19) + counter.to_bytes(4, "big") + bytes([last])
        output = ctypes.create_string_buffer(len(plaintext) + 16)
        length = ctypes.c_ulonglong()
        status = sodium.crypto_aead_xchacha20poly1305_ietf_encrypt(
            output, ctypes.byref(length), plaintext, len(plaintext), aad, len(aad),
            None, nonce, bytes([0x11] * 32),
        )
        if status != 0 or length.value != len(output) or output.raw.hex() != expected:
            raise SystemExit("Reference STREAM fixture mismatch")
        print(f"STREAM segment {counter}: {output.raw.hex()}")


if __name__ == "__main__":
    main()
