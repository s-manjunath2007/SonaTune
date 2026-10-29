#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char *argv[]) {
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-V") == 0 || strcmp(argv[i], "--version") == 0) {
            printf("GNU windres 2.44\n");
            return 0;
        }
        if (strcmp(argv[i], "/?") == 0 || strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            printf("Usage: windres [options]\n");
            return 0;
        }
    }

    char *out_path = NULL;
    for (int i = 1; i < argc; i++) {
        if ((strcmp(argv[i], "-o") == 0 || strcmp(argv[i], "--output") == 0) && i + 1 < argc) {
            out_path = argv[++i];
        } else if (strncmp(argv[i], "--output=", 9) == 0) {
            out_path = argv[i] + 9;
        } else if (strncmp(argv[i], "-o=", 3) == 0) {
            out_path = argv[i] + 3;
        }
    }

    if (out_path) {
        FILE *out = fopen(out_path, "wb");
        if (out) {
            fwrite("!<arch>\n", 1, 8, out);
            fclose(out);
        }
    }
    return 0;
}
