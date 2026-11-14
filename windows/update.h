static bool launchCheck(bool manual){
 wchar_t app[MAX_PATH],script[MAX_PATH],system[MAX_PATH],args[4096];
 if(!GetModuleFileNameW(NULL,app,MAX_PATH)||!GetSystemDirectoryW(system,MAX_PATH))return false;
 wcscpy(script,app);wchar_t *end=wcsrchr(script,L'\\');if(!end || end-script+13>=MAX_PATH)return false;
 wcscpy(end,L"\\update.ps1");
 const wchar_t *suffix=L"\\WindowsPowerShell\\v1.0\\powershell.exe";
 if(wcslen(system)+wcslen(suffix)>=MAX_PATH)return false;wcscat(system,suffix);
 swprintf(args,4096,L"\"%ls\" -NoProfile -NonInteractive -STA -ExecutionPolicy Bypass -File \"%ls\" %ls -PlayerId %lu -CurrentVersion %ls -PlayerPath \"%ls\"",system,script,manual?L"-Manual":L"",GetCurrentProcessId(),RMP_VERSION,app);
 if(GetFileAttributesW(script)==INVALID_FILE_ATTRIBUTES)return false;
 STARTUPINFOW start={sizeof(start)};PROCESS_INFORMATION process;
 if(CreateProcessW(system,args,NULL,NULL,FALSE,CREATE_NO_WINDOW,NULL,NULL,&start,&process)){CloseHandle(process.hThread);CloseHandle(process.hProcess);return true;}
 return false;
}
