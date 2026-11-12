/* Prints the CRC-16/ARC of stdin (or of FILE from OFFSET to EOF) in hex. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
  uint16_t table[256];
  for (int i = 0; i < 256; i++) {
    uint16_t c = i;
    for (int k = 0; k < 8; k++) c = (c & 1) ? (c >> 1) ^ 0xA001 : c >> 1;
    table[i] = c;
  }

  FILE *f = stdin;
  if (argc > 1) {
    f = fopen(argv[1], "rb");
    if (!f) { perror(argv[1]); return 1; }
    if (argc > 2 && fseeko(f, strtoll(argv[2], NULL, 10), SEEK_SET)) { perror("seek"); return 1; }
  }

  static unsigned char buf[1 << 20];
  uint16_t crc = 0;
  size_t n;
  while ((n = fread(buf, 1, sizeof buf, f)) > 0)
    for (size_t i = 0; i < n; i++) crc = (crc >> 8) ^ table[(crc ^ buf[i]) & 0xff];
  if (ferror(f)) { perror(argc > 1 ? argv[1] : "stdin"); return 1; }
  printf("%04x\n", crc);
  return 0;
}
