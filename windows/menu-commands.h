/* Handles the VLC-style menu commands. Mirrors macOS runMenuCommand in mac-menus.h. */
static void askAndRun(const WCHAR *title,const WCHAR *message,const char *key,const WCHAR *initial){
 WCHAR value[1024];
 if(!askValue(title,message,initial,value,1024))return;
 engineWide(key,value,title);
}
static void saveAndRun(const WCHAR *title,const WCHAR *filter,const WCHAR *ext,const char *key){
 WCHAR path[MAX_PATH]=L"";
 if(!pickPath(true,title,filter,ext,path,MAX_PATH))return;
 engineWide(key,path,title);
}
static void timeCommand(void){
 WCHAR value[64];
 if(!askValue(L"Jump to Specific Time",L"Enter elapsed seconds or a time such as 1:30.",L"0",value,64))return;
 double seconds=0;WCHAR *cursor=value;
 while(*cursor){
  wchar_t *end=NULL;double part=wcstod(cursor,&end);
  if(end==cursor||part<0){MessageBoxW(mainwin,L"Enter a valid playback time.",L"Jump to Specific Time",MB_ICONWARNING);return;}
  seconds=seconds*60+part;
  if(!*end)break;
  if(*end!=L':'){MessageBoxW(mainwin,L"Enter a valid playback time.",L"Jump to Specific Time",MB_ICONWARNING);return;}
  cursor=end+1;
 }
 char text[64];snprintf(text,sizeof(text),"%.3f",seconds);
 engineAction("time",text,L"Jump to Specific Time",false);
}
static bool menuCommand(HWND w,int id){
 switch(id){
 case M_MULTI:openMultiple();return true;
 case M_FOLDER:chooseFolder();return true;
 case M_DISC:discLocation();return true;
 case M_CAPTURE:showText(L"Open Capture Device",L"Capture devices are not available in this version of RMP on Windows.");return true;
 case M_CLIPBOARD:clipboardLocation();return true;
 case M_CONVERT:saveAndRun(L"Convert / Save",L"MP4 video\0*.mp4\0All files\0*.*\0",L"mp4","convert");return true;
 case M_STREAM_OUT:askAndRun(L"Stream",L"Stream the current media to a UDP receiver.","stream",L"udp://127.0.0.1:1234");return true;
 case M_END_OUTPUT:engineAction("output-stop","",L"Stop Media Output",false);return true;
 case M_QUIT_END:quitAtEnd=!quitAtEnd;return true;
 case M_QUIT:PostMessageW(w,WM_CLOSE,0,0);return true;
 case M_FWD:engineAction("jump","10",L"Jump Forward",false);return true;
 case M_BACK:engineAction("jump","-10",L"Jump Backward",false);return true;
 case M_TIME:timeCommand();return true;
 case M_RECORD:
  if(recording){if(engineAction("record","stop",L"Record",false))recording=false;return true;}
  {WCHAR path[MAX_PATH]=L"";
   if(!pickPath(true,L"Record to File",L"Transport stream\0*.ts\0All files\0*.*\0",L"ts",path,MAX_PATH))return true;
   if(engineWide("record",path,L"Record"))recording=true;}
  return true;
 case M_VOL_UP:applyVolume(volumePercent+5);return true;
 case M_VOL_DOWN:applyVolume(volumePercent-5);return true;
 case M_MUTE:engineAction("mute","",L"Mute",false);return true;
 case M_WALL:wallpaperMode=setVideoWallpaper(!wallpaperMode);return true;
 case M_FIT:engineAction("zoom","0",L"Always Fit Window",false);return true;
 case M_SNAPSHOT:saveAndRun(L"Take Snapshot",L"PNG image\0*.png\0All files\0*.*\0",L"png","snapshot");return true;
 case M_SUBFILE:{WCHAR path[MAX_PATH]=L"";
   if(pickPath(false,L"Add Subtitle File",L"Subtitles\0*.srt;*.vtt;*.ass;*.ssa;*.sub;*.idx;*.ttml;*.smi\0All files\0*.*\0",NULL,path,MAX_PATH))engineWide("subtitle-file",path,L"Add Subtitle File");}
  return true;
 case M_EFFECTS:DialogBoxParamW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(250),w,effectsProc,0);return true;
 case M_SYNC:DialogBoxParamW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(240),w,effectsProc,1);return true;
 case M_CUSTOM:case M_PREFS:DialogBoxParamW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(230),w,preferencesProc,0);return true;
 case M_QUEUE:playlistVisible=!playlistVisible;syncQueueWindow();return true;
 case M_DOCK:dockUI=!dockUI;relayoutPlayer();return true;
 case M_TOP:topUI=!topUI;SetWindowPos(w,topUI?HWND_TOPMOST:HWND_NOTOPMOST,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE);return true;
 case M_MINIMAL:minimalUI=!minimalUI;relayoutPlayer();return true;
 case M_ADVANCED:advancedUI=!advancedUI;relayoutPlayer();return true;
 case M_STATUS:statusUI=!statusUI;relayoutPlayer();return true;
 case M_INTERFACE:revealTerminal();return true;
 case M_VLSUB:{
   if(MessageBoxW(w,L"Search for subtitles on OpenSubtitles using the current media title. After downloading a subtitle, choose Subtitle → Add Subtitle File.",L"Search for Subtitles",MB_OKCANCEL|MB_ICONINFORMATION)!=IDOK)return true;
   ShellExecuteW(w,L"open",L"https://www.opensubtitles.org/",NULL,NULL,SW_SHOWNORMAL);}
  return true;
 case M_HELP:ShellExecuteW(w,L"open",L"https://github.com/KMThegamer2022/RMP#readme",NULL,NULL,SW_SHOWNORMAL);return true;
 case M_ABOUT:showText(L"About RMP",L"RMP — Rust Media Player.\nhttps://github.com/KMThegamer2022/RMP");return true;
 }
 return false;
}
