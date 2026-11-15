/* Native Windows menus. Option IDs come from the same playback engine used by
   the macOS UI; submenu selections never depend on localized display text. */
typedef struct {HMENU menu;const WCHAR *label;const char *key;} MediaMenu;
static MediaMenu optionMenus[]={
 {NULL,L"Title","title"},{NULL,L"Chapter","chapter"},{NULL,L"Programme","programme"},{NULL,L"Custom Bookmarks","bookmark"},
 {NULL,L"Renderer","renderer"},{NULL,L"Speed","speed"},{NULL,L"Audio Track","audio-track"},{NULL,L"Audio Device","audio-device"},
 {NULL,L"Stereo Mode","stereo"},{NULL,L"Visualizations","visualization"},{NULL,L"Video Track","video-track"},
 {NULL,L"Zoom","zoom"},{NULL,L"Aspect Ratio","aspect"},{NULL,L"Crop","crop"},{NULL,L"Deinterlace","deinterlace"},
 {NULL,L"Deinterlace Mode","deinterlace-mode"},{NULL,L"Sub Track","subtitle-track"}
};
static HMENU recentMenu,mediaMenu,playMenu,viewMenu;
static HACCEL menuKeys;
static void menuLine(HMENU m,int id,const WCHAR *label){AppendMenuW(m,MF_STRING,(UINT_PTR)id,label);}
static void divider(HMENU m){AppendMenuW(m,MF_SEPARATOR,0,NULL);}
static HMENU basicMenu(HMENU parent,const WCHAR *label){
 HMENU m=CreatePopupMenu();AppendMenuW(parent,MF_POPUP,(UINT_PTR)m,label);return m;
}
static void optionMenu(HMENU parent,size_t index){
 MediaMenu *o=&optionMenus[index];o->menu=submenu(parent,o->label);
}
static void clearOptionIds(HMENU m){
 while(GetMenuItemCount(m)>0){
  MENUITEMINFOW item={sizeof(item)};item.fMask=MIIM_DATA;
  if(GetMenuItemInfoW(m,0,TRUE,&item))free((void*)item.dwItemData);
  DeleteMenu(m,0,MF_BYPOSITION);
 }
}
static void putOption(HMENU menu,const char *value,const char *text){
 WCHAR *label=wide(text);char *copy=_strdup(value);
 if(label&&copy){MENUITEMINFOW item={sizeof(item)};item.fMask=MIIM_STRING|MIIM_DATA;item.dwTypeData=label;item.dwItemData=(ULONG_PTR)copy;
  if(InsertMenuItemW(menu,GetMenuItemCount(menu),TRUE,&item))copy=NULL;
 }
 free(label);free(copy);
}
static bool fillPlayerMenu(HMENU m){
 for(size_t i=0;i<sizeof(optionMenus)/sizeof(*optionMenus);i++)if(optionMenus[i].menu==m){
  clearOptionIds(m);char *options=ap_menu_options(player,optionMenus[i].key);
  if(options){
   char *line=options;
   while(*line){char *next=strchr(line,'\n');if(next)*next++=0;char *tab=strchr(line,'\t');
    if(tab){*tab++=0;putOption(m,line,tab);}if(!next)break;line=next;
   }
   ap_terminal_free(options);
  }
  if(!GetMenuItemCount(m))AppendMenuW(m,MF_GRAYED,0,L"Unavailable for this media");
  return true;
 }
 if(m==recentMenu){
  clearOptionIds(m);size_t n=ap_library_count(player);
  for(size_t i=0;i<min(n,12);i++){
   const char *path=ap_library_item(player,n-i-1);if(path)putOption(m,path,path);
  }
  if(!GetMenuItemCount(m))AppendMenuW(m,MF_GRAYED,0,L"No recent media");
  return true;
 }
 return false;
}
typedef struct {const WCHAR *title,*message,*value;WCHAR *result;size_t capacity;} MenuPrompt;
static INT_PTR CALLBACK promptProc(HWND d,UINT m,WPARAM w,LPARAM l){
 if(m==WM_INITDIALOG){MenuPrompt *p=(MenuPrompt*)l;SetWindowLongPtrW(d,DWLP_USER,l);SetWindowTextW(d,p->title);
  SetDlgItemTextW(d,211,p->message);SetDlgItemTextW(d,212,p->value?p->value:L"");SetFocus(GetDlgItem(d,212));SendDlgItemMessageW(d,212,EM_SETSEL,0,-1);return FALSE;}
 if(m==WM_COMMAND&&LOWORD(w)==IDOK){MenuPrompt *p=(MenuPrompt*)GetWindowLongPtrW(d,DWLP_USER);GetDlgItemTextW(d,212,p->result,(int)p->capacity);EndDialog(d,IDOK);return TRUE;}
 if(m==WM_COMMAND&&LOWORD(w)==IDCANCEL){EndDialog(d,IDCANCEL);return TRUE;}return FALSE;
}
static bool askValue(const WCHAR *title,const WCHAR *message,const WCHAR *value,WCHAR *out,size_t capacity){
 MenuPrompt prompt={title,message,value,out,capacity};return DialogBoxParamW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(210),mainwin,promptProc,(LPARAM)&prompt)==IDOK;
}
static INT_PTR CALLBACK infoProc(HWND d,UINT m,WPARAM w,LPARAM l){
 if(m==WM_INITDIALOG){MenuPrompt *p=(MenuPrompt*)l;SetWindowTextW(d,p->title);
  size_t n=wcslen(p->message);WCHAR *body=malloc((2*n+1)*sizeof(WCHAR));if(body){size_t j=0;for(size_t i=0;i<n;i++){if(p->message[i]==L'\n'&&(!i||p->message[i-1]!=L'\r'))body[j++]=L'\r';body[j++]=p->message[i];}body[j]=0;SetDlgItemTextW(d,221,body);free(body);}return TRUE;}
 if(m==WM_COMMAND&&(LOWORD(w)==IDOK||LOWORD(w)==IDCANCEL)){EndDialog(d,0);return TRUE;}return FALSE;
}
static void showText(const WCHAR *title,const WCHAR *message){MenuPrompt p={title,message,NULL,NULL,0};DialogBoxParamW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(220),mainwin,infoProc,(LPARAM)&p);}
static bool engineAction(const char *key,const char *value,const WCHAR *title,bool report){
 char *out=ap_menu_action(player,key,value);bool empty=out&&!*out;
 if(!out){MessageBoxW(mainwin,L"The media engine could not perform this action.",title,MB_ICONERROR);return false;}
 if(*out||report){WCHAR *message=wide(*out?out:"No information available.");if(message)showText(title,message);free(message);}
 ap_terminal_free(out);return empty;
}
static bool engineWide(const char *key,const WCHAR *value,const WCHAR *title){
 char *utf8=terminalUtf8(value);if(!utf8)return false;bool ok=engineAction(key,utf8,title,false);free(utf8);return ok;
}
static bool selectMenuOption(HMENU m,UINT position){
 if(m!=recentMenu){bool known=false;for(size_t i=0;i<sizeof(optionMenus)/sizeof(*optionMenus);i++)if(optionMenus[i].menu==m)known=true;if(!known)return false;}
 MENUITEMINFOW item={sizeof(item)};item.fMask=MIIM_DATA;
 if(!GetMenuItemInfoW(m,position,TRUE,&item)||!item.dwItemData)return true;
 const char *value=(const char*)item.dwItemData;
 if(m==recentMenu){WCHAR *path=wide(value);if(path){enqueue(path,strstr(value,"://")!=NULL);start(ap_count(player)-1);free(path);}}
 else for(size_t i=0;i<sizeof(optionMenus)/sizeof(*optionMenus);i++)if(optionMenus[i].menu==m){engineAction(optionMenus[i].key,value,optionMenus[i].label,false);break;}
 return true;
}
static void createPlayerMenus(HWND w){
 bar=CreateMenu();mediaMenu=basicMenu(bar,L"&Media");
 menuLine(mediaMenu,OPEN,L"Open &File…\tCtrl+O");menuLine(mediaMenu,M_MULTI,L"Open Multiple Files…\tCtrl+Shift+O");
 menuLine(mediaMenu,M_FOLDER,L"Open Folder…\tCtrl+F");menuLine(mediaMenu,M_DISC,L"Open Disc…\tCtrl+D");
 menuLine(mediaMenu,STREAM,L"Open Network Stream…\tCtrl+N");menuLine(mediaMenu,M_CAPTURE,L"Open Capture Device…\tCtrl+C");
 menuLine(mediaMenu,M_CLIPBOARD,L"Open Location from Clipboard\tCtrl+V");recentMenu=submenu(mediaMenu,L"Open Recent Media");divider(mediaMenu);
 menuLine(mediaMenu,SAVE_LIST,L"Save Playlist to File…\tCtrl+Y");divider(mediaMenu);
 menuLine(mediaMenu,M_CONVERT,L"Convert / Save…\tCtrl+R");menuLine(mediaMenu,M_STREAM_OUT,L"Stream…\tCtrl+S");menuLine(mediaMenu,M_END_OUTPUT,L"Stop Media Output");divider(mediaMenu);
 menuLine(mediaMenu,M_QUIT_END,L"Quit at the End of Playlist");menuLine(mediaMenu,M_QUIT,L"Quit\tCtrl+Q");
 playMenu=basicMenu(bar,L"&Playback");
 for(int i=0;i<6;i++){if(i==4||i==5)divider(playMenu);optionMenu(playMenu,i);}
 menuLine(playMenu,M_FWD,L"Jump Forward");menuLine(playMenu,M_BACK,L"Jump Backward");menuLine(playMenu,M_TIME,L"Jump to Specific Time…\tCtrl+T");divider(playMenu);
 menuLine(playMenu,PLAY,L"Play / Pause\tSpace");menuLine(playMenu,STOP,L"Stop");menuLine(playMenu,PREV,L"Previous");menuLine(playMenu,NEXT,L"Next");menuLine(playMenu,M_RECORD,L"Record…");
 HMENU a=basicMenu(bar,L"&Audio");optionMenu(a,6);optionMenu(a,7);optionMenu(a,8);divider(a);optionMenu(a,9);divider(a);
 menuLine(a,M_VOL_UP,L"Increase Volume\tCtrl+Up");menuLine(a,M_VOL_DOWN,L"Decrease Volume\tCtrl+Down");menuLine(a,M_MUTE,L"Mute");
 HMENU v=basicMenu(bar,L"&Video");optionMenu(v,10);divider(v);menuLine(v,FULL,L"Fullscreen");menuLine(v,M_FIT,L"Always Fit Window");menuLine(v,M_WALL,L"Set as Wallpaper");divider(v);
 optionMenu(v,11);optionMenu(v,12);optionMenu(v,13);divider(v);optionMenu(v,14);optionMenu(v,15);divider(v);menuLine(v,M_SNAPSHOT,L"Take Snapshot…");
 HMENU sub=basicMenu(bar,L"&Subtitle");menuLine(sub,M_SUBFILE,L"Add Subtitle File…");optionMenu(sub,16);
 HMENU tools=basicMenu(bar,L"&Tools");menuLine(tools,M_EFFECTS,L"Effects and Filters…\tCtrl+E");menuLine(tools,M_SYNC,L"Track Synchronization…");
 menuLine(tools,M_INFO,L"Media Information\tCtrl+I");menuLine(tools,M_CODEC,L"Codec Information\tCtrl+J");menuLine(tools,M_VLM,L"VLM Configuration…\tCtrl+Shift+W");
 menuLine(tools,M_GUIDE,L"Program Guide");menuLine(tools,M_MESSAGES,L"Messages\tCtrl+M");menuLine(tools,M_PLUGINS,L"Plugins and Extensions");
 divider(tools);menuLine(tools,M_CUSTOM,L"Customise Interface…");menuLine(tools,M_PREFS,L"Preferences…\tCtrl+P");
 viewMenu=basicMenu(bar,L"V&iew");menuLine(viewMenu,M_QUEUE,L"Playlist\tCtrl+L");menuLine(viewMenu,M_DOCK,L"Docked Playlist");divider(viewMenu);
 menuLine(viewMenu,M_TOP,L"Always on Top");divider(viewMenu);menuLine(viewMenu,M_MINIMAL,L"Minimal Interface\tCtrl+H");menuLine(viewMenu,FULL,L"Fullscreen Interface\tF11");
 menuLine(viewMenu,M_ADVANCED,L"Advanced Controls");menuLine(viewMenu,M_STATUS,L"Status Bar");divider(viewMenu);
 extraTerminalMenu=basicMenu(viewMenu,L"Add Interface");menuLine(extraTerminalMenu,M_INTERFACE,L"RMP Terminal");
 menuLine(viewMenu,M_VLSUB,L"VLsub — Find Subtitles Online…");
 themeMenu=basicMenu(viewMenu,L"Theme");int order[]={VLC_THEME,RMP_DEFAULT,MODERN,SEVEN,XP,MAC,LINUX_THEME};
 for(size_t i=0;i<sizeof(order)/sizeof(*order);i++)menuLine(themeMenu,THEME_FIRST+order[i],names[order[i]]);
 queue=submenu(viewMenu,L"Play from Queue");menuLine(viewMenu,SHOW_TERMINAL,L"RMP Terminal");
 HMENU help=basicMenu(bar,L"&Help");menuLine(help,M_HELP,L"Help\tF1");menuLine(help,490,L"Check for Updates…");divider(help);menuLine(help,M_ABOUT,L"About RMP\tShift+F1");
 SetMenu(w,bar);
 #define CTRLKEY(k,id) {FVIRTKEY|FCONTROL,k,id}
 ACCEL keys[]={CTRLKEY('O',OPEN),{FVIRTKEY|FCONTROL|FSHIFT,'O',M_MULTI},CTRLKEY('F',M_FOLDER),CTRLKEY('D',M_DISC),CTRLKEY('N',STREAM),
 CTRLKEY('C',M_CAPTURE),CTRLKEY('V',M_CLIPBOARD),CTRLKEY('Y',SAVE_LIST),CTRLKEY('R',M_CONVERT),CTRLKEY('S',M_STREAM_OUT),CTRLKEY('Q',M_QUIT),
 CTRLKEY('T',M_TIME),CTRLKEY('E',M_EFFECTS),CTRLKEY('I',M_INFO),CTRLKEY('J',M_CODEC),{FVIRTKEY|FCONTROL|FSHIFT,'W',M_VLM},CTRLKEY('M',M_MESSAGES),
 CTRLKEY('P',M_PREFS),CTRLKEY('L',M_QUEUE),CTRLKEY('H',M_MINIMAL),CTRLKEY(VK_UP,M_VOL_UP),CTRLKEY(VK_DOWN,M_VOL_DOWN),
 {FVIRTKEY,VK_F1,M_HELP},{FVIRTKEY|FSHIFT,VK_F1,M_ABOUT},{FVIRTKEY,VK_SPACE,PLAY}};
 #undef CTRLKEY
 menuKeys=CreateAcceleratorTableW(keys,sizeof(keys)/sizeof(*keys));
}
