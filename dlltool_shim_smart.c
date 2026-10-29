#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>

int main(int argc, char *argv[]) {
    char *def_file = NULL;
    char *lib_file = NULL;
    char *dll_name = "output.dll";
    char *machine = "/machine:x64";

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-d") == 0 && i + 1 < argc) {
            def_file = argv[++i];
        } else if (strcmp(argv[i], "-l") == 0 && i + 1 < argc) {
            lib_file = argv[++i];
        } else if (strcmp(argv[i], "-D") == 0 && i + 1 < argc) {
            dll_name = argv[++i];
        } else if (strcmp(argv[i], "-m") == 0 && i + 1 < argc) {
            i++;
            if (strstr(argv[i], "64") != NULL) {
                machine = "/machine:x64";
            } else {
                machine = "/machine:x86";
            }
        }
    }

    if (!def_file || !lib_file) {
        return 0;
    }

    char abs_def[MAX_PATH];
    char abs_lib[MAX_PATH];
    GetFullPathNameA(def_file, MAX_PATH, abs_def, NULL);
    GetFullPathNameA(lib_file, MAX_PATH, abs_lib, NULL);

    // Check if we have a genuine GNU import library for this DLL
    char base_name[128];
    strncpy(base_name, dll_name, sizeof(base_name) - 1);
    base_name[sizeof(base_name) - 1] = '\0';
    char *dot = strrchr(base_name, '.');
    if (dot && _stricmp(dot, ".dll") == 0) {
        *dot = '\0';
    }

    char gnu_lib_path[MAX_PATH];
    snprintf(gnu_lib_path, sizeof(gnu_lib_path),
             "C:\\Users\\S MANJUNATH\\.rustup\\toolchains\\stable-x86_64-pc-windows-gnu\\lib\\rustlib\\x86_64-pc-windows-gnu\\lib\\self-contained\\lib%s.a",
             base_name);

    if (GetFileAttributesA(gnu_lib_path) != INVALID_FILE_ATTRIBUTES) {
        if (CopyFileA(gnu_lib_path, abs_lib, FALSE)) {
            return 0;
        }
    }

    // Fallback to lld-link for DLLs not in the GNU sysroot (e.g. bcryptprimitives.dll)
    const char *lld_dir = "C:\\Users\\S MANJUNATH\\.rustup\\toolchains\\stable-x86_64-pc-windows-gnu\\lib\\rustlib\\x86_64-pc-windows-gnu\\bin\\gcc-ld";
    const char *lld_exe = "C:\\Users\\S MANJUNATH\\.rustup\\toolchains\\stable-x86_64-pc-windows-gnu\\lib\\rustlib\\x86_64-pc-windows-gnu\\bin\\gcc-ld\\lld-link.exe";

    char cmd_line[4096];
    snprintf(cmd_line, sizeof(cmd_line), "\"%s\" /def:\"%s\" /out:\"%s\" /implib:\"%s\" %s /nologo",
             lld_exe, abs_def, dll_name, abs_lib, machine);

    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    ZeroMemory(&pi, sizeof(pi));

    if (!CreateProcessA(NULL, cmd_line, NULL, NULL, FALSE, 0, NULL, lld_dir, &si, &pi)) {
        return 1;
    }

    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD exit_code = 0;
    GetExitCodeProcess(pi.hProcess, &exit_code);
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);

    // Delete dummy output DLL in lld_dir or current dir
    char dummy_dll[MAX_PATH];
    snprintf(dummy_dll, sizeof(dummy_dll), "%s\\%s", lld_dir, dll_name);
    DeleteFileA(dummy_dll);
    DeleteFileA(dll_name);

    return (int)exit_code;
}
