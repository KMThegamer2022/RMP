static bool pickPath(bool save,const WCHAR *title,const WCHAR *filter,const WCHAR *ext,WCHAR *path,DWORD size){
 OPENFILENAMEW dlg={sizeof(dlg)};dlg.hwndOwner=mainwin;dlg.lpstrTitle=title;dlg.lpstrFilter=filter;dlg.lpstrDefExt=ext;
 dlg.lpstrFile=path;dlg.nMaxFile=size;dlg.Flags=OFN_PATHMUSTEXIST|OFN_NOCHANGEDIR|(save?OFN_OVERWRITEPROMPT:OFN_FILEMUSTEXIST);
 return save?GetSaveFileNameW(&dlg):GetOpenFileNameW(&dlg);
}
static WCHAR *joinPath(const WCHAR *a,const WCHAR *b){
 size_t n=wcslen(a)+wcslen(b)+2;WCHAR *out=malloc(n*sizeof(WCHAR));
 if(out)swprintf(out,n,L"%ls%ls%ls",a,a[0]&&a[wcslen(a)-1]==L'\\'?L"":L"\\",b);
 return out;
}
static void openMultiple(void){
 ap_prepare(player);DWORD cap=65536;WCHAR *buffer=calloc(cap,sizeof(WCHAR));if(!buffer)return;
 OPENFILENAMEW f={sizeof(f)};f.hwndOwner=mainwin;f.lpstrTitle=L"Open Multiple Files";f.lpstrFilter=L"Media files\0*.*\0";f.lpstrFile=buffer;f.nMaxFile=cap;
 f.Flags=OFN_FILEMUSTEXIST|OFN_NOCHANGEDIR|OFN_EXPLORER|OFN_ALLOWMULTISELECT;
 if(GetOpenFileNameW(&f)){
  size_t startAt=ap_count(player);WCHAR *name=buffer+wcslen(buffer)+1;
  if(!*name)enqueue(buffer,false);
  else while(*name){WCHAR *path=joinPath(buffer,name);if(path){enqueue(path,false);free(path);}name+=wcslen(name)+1;}
  start(startAt);
 }free(buffer);
}
enum { FOLDER_READY=WM_APP+93 };
typedef struct {WCHAR **paths;size_t count,capacity;bool failed;HWND owner;WCHAR *root;} FolderMedia;
static bool mediaExtension(const WCHAR *path){
 const WCHAR *ext=wcsrchr(path,L'.');if(!ext||wcslen(ext)>12)return false;
 WCHAR token[20];swprintf(token,20,L"|%ls|",ext+1);CharLowerBuffW(token,lstrlenW(token));
 return wcsstr(L"|3g2|3gp|aac|ac3|aif|aifc|aiff|alac|amr|aob|ape|asf|au|avi|axa|axv|bik|caf|crf|divx|dts|dvdmedia|eac3|evo|f4v|flac|flv|gxf|it|m1v|m2p|m2ts|m2v|m4a|m4b|m4p|m4v|mid|midi|mk3d|mka|mks|mkv|mlp|mod|mov|mp3|mp4|mpa|mpc|mpeg|mpeg2|mpeg4|mpg|mts|mxf|nsv|nuv|oga|ogg|ogm|ogv|oma|opus|rec|rmi|s3m|spx|tak|tod|ts|tta|vob|voc|vqf|vro|wav|webm|wma|wmv|wtv|wv|xa|xesc|xm|",token)!=NULL;
}
static void readFolder(FolderMedia *batch,const WCHAR *directory){
 WCHAR *mask=joinPath(directory,L"*");if(!mask){batch->failed=true;return;}
 WIN32_FIND_DATAW data;HANDLE find=FindFirstFileW(mask,&data);free(mask);if(find==INVALID_HANDLE_VALUE)return;
 do{
  if(!wcscmp(data.cFileName,L".")||!wcscmp(data.cFileName,L".."))continue;
  WCHAR *path=joinPath(directory,data.cFileName);if(!path){batch->failed=true;break;}
  if(data.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY){
   if(!(data.dwFileAttributes&FILE_ATTRIBUTE_REPARSE_POINT))readFolder(batch,path);
   free(path);
  }else if(mediaExtension(path)){
   if(batch->count==batch->capacity){
    size_t cap=batch->capacity?2*batch->capacity:128;WCHAR **p=realloc(batch->paths,cap*sizeof(*p));
    if(!p){batch->failed=true;free(path);break;}batch->paths=p;batch->capacity=cap;
   }
   batch->paths[batch->count++]=path;
  }else free(path);
 }while(!batch->failed&&FindNextFileW(find,&data));
 FindClose(find);
}
static int sortPaths(const void *a,const void *b){return _wcsicmp(*(WCHAR *const*)a,*(WCHAR *const*)b);}
static void freeFolder(FolderMedia *batch){
 for(size_t i=0;i<batch->count;i++)free(batch->paths[i]);free(batch->paths);free(batch->root);free(batch);
}
static DWORD WINAPI loadFolder(void *arg){
 FolderMedia *batch=arg;readFolder(batch,batch->root);
 if(batch->count)qsort(batch->paths,batch->count,sizeof(*batch->paths),sortPaths);
 if(!PostMessageW(batch->owner,FOLDER_READY,0,(LPARAM)batch))freeFolder(batch);return 0;
}
static void chooseFolder(void){
 BROWSEINFOW b={0};b.hwndOwner=mainwin;b.lpszTitle=L"Choose a media folder";b.ulFlags=BIF_NEWDIALOGSTYLE|BIF_RETURNONLYFSDIRS;
 PIDLIST_ABSOLUTE folder=SHBrowseForFolderW(&b);if(!folder)return;
 PWSTR path=NULL;SHGetNameFromIDList(folder,SIGDN_FILESYSPATH,&path);CoTaskMemFree(folder);if(!path)return;
 FolderMedia *batch=calloc(1,sizeof(*batch));
 if(batch){batch->root=_wcsdup(path);batch->owner=mainwin;
  if(!batch->root||!QueueUserWorkItem(loadFolder,batch,WT_EXECUTEDEFAULT)){freeFolder(batch);MessageBoxW(mainwin,L"Could not read the folder.",L"Open Folder",MB_ICONERROR);}
 }
 CoTaskMemFree(path);
}
static void receiveFolder(FolderMedia *batch){
 size_t first=ap_count(player);
 if(batch->failed||!batch->count)MessageBoxW(mainwin,batch->failed?L"Could not finish reading this folder.":L"No supported media files were found.",L"Open Folder",MB_OK);
 else{for(size_t i=0;i<batch->count;i++)enqueue(batch->paths[i],false);start(first);}
 freeFolder(batch);
}
static void playLocation(const WCHAR *location){
 if(!location||!*location)return;bool url=wcsstr(location,L"://")!=NULL;
 if(!url&&GetFileAttributesW(location)==INVALID_FILE_ATTRIBUTES){MessageBoxW(mainwin,L"Choose an existing file or enter a full stream URL.",L"Open Location",MB_ICONERROR);return;}
 enqueue(location,url);start(ap_count(player)-1);
}
static void clipboardLocation(void){
 if(!OpenClipboard(mainwin))return;
 HANDLE data=GetClipboardData(CF_UNICODETEXT);const WCHAR *text=data?GlobalLock(data):NULL;
 WCHAR *copy=text?_wcsdup(text):NULL;if(text)GlobalUnlock(data);CloseClipboard();
 WCHAR result[32768]={0};if(askValue(L"Open Location from Clipboard",L"File or stream URL",copy,result,32768))playLocation(result);free(copy);
}
static void discLocation(void){
 WCHAR drives[256];WCHAR proposed[256]=L"dvd:///D:/";GetLogicalDriveStringsW(255,drives);
 for(WCHAR *d=drives;*d;d+=wcslen(d)+1)if(GetDriveTypeW(d)==DRIVE_CDROM){swprintf(proposed,256,L"dvd:///%lc:/",d[0]);break;}
 WCHAR out[32768]={0};if(askValue(L"Open Disc",L"Disc location (dvd:///, bluray:/// or cdda:///)",proposed,out,32768))playLocation(out);
}
static void applyVolume(int v){
 volumePercent=max(0,min(100,v));ap_volume(player,volumePercent);SendMessageW(level,TBM_SETPOS,TRUE,volumePercent);
}
static WCHAR syncAudio[48]=L"0",syncSubtitle[48]=L"0",brightness[48]=L"1",contrast[48]=L"1";
static void applyViewFlags(void){
 SetWindowPos(mainwin,topUI?HWND_TOPMOST:HWND_NOTOPMOST,0,0,0,0,SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE);
 if(queueWindow)recreateQueueWindow();relayoutPlayer();
}
static INT_PTR CALLBACK preferencesProc(HWND d,UINT m,WPARAM w,LPARAM l){
 if(m==WM_INITDIALOG){
  SetWindowTextW(d,l?L"Customise Interface":L"Preferences");
  for(int i=0;i<THEME_COUNT;i++)SendDlgItemMessageW(d,231,CB_ADDSTRING,0,(LPARAM)names[i]);
  SendDlgItemMessageW(d,231,CB_SETCURSEL,skin,0);SetDlgItemInt(d,232,volumePercent,FALSE);
  CheckDlgButton(d,233,topUI);CheckDlgButton(d,234,statusUI);CheckDlgButton(d,235,advancedUI);CheckDlgButton(d,236,dockUI);return TRUE;
 }
 if(m==WM_COMMAND&&LOWORD(w)==IDOK){
  BOOL valid;UINT value=GetDlgItemInt(d,232,&valid,FALSE);if(!valid||value>100){MessageBoxW(d,L"Enter a volume from 0 to 100.",L"Preferences",MB_OK);return TRUE;}
  int choice=(int)SendDlgItemMessageW(d,231,CB_GETCURSEL,0,0);applyVolume(value);
  topUI=IsDlgButtonChecked(d,233)==BST_CHECKED;statusUI=IsDlgButtonChecked(d,234)==BST_CHECKED;
  advancedUI=IsDlgButtonChecked(d,235)==BST_CHECKED;dockUI=IsDlgButtonChecked(d,236)==BST_CHECKED;
  DWORD flags=(topUI?1:0)|(statusUI?2:0)|(advancedUI?4:0)|(dockUI?8:0);
  RegSetKeyValueW(HKEY_CURRENT_USER,L"Software\\RMP",L"InterfaceOptions",REG_DWORD,&flags,sizeof(flags));
  RegSetKeyValueW(HKEY_CURRENT_USER,L"Software\\RMP",L"DefaultVolume",REG_DWORD,&value,sizeof(value));
  if(choice>=0&&choice<THEME_COUNT)SendMessageW(mainwin,WM_COMMAND,THEME_FIRST+choice,0);
  applyViewFlags();EndDialog(d,IDOK);return TRUE;
 }
 if(m==WM_COMMAND&&LOWORD(w)==IDCANCEL){EndDialog(d,0);return TRUE;}return FALSE;
}
static bool readNumber(HWND d,int id,WCHAR *out,int cap,double minValue,double maxValue){
 GetDlgItemTextW(d,id,out,cap);WCHAR *end=NULL;double value=wcstod(out,&end);
 if(end==out||*end||!isfinite(value)||value<minValue||value>maxValue){MessageBoxW(d,L"Enter a number in the displayed range.",L"RMP",MB_ICONWARNING);return false;}return true;
}
static INT_PTR CALLBACK effectsProc(HWND d,UINT m,WPARAM w,LPARAM l){
 bool effects=GetDlgItem(d,251)!=NULL;
 if(m==WM_INITDIALOG){
  if(effects){
   SendDlgItemMessageW(d,251,CB_ADDSTRING,0,(LPARAM)L"Keep current equalizer");
   char *options=ap_menu_options(player,"equalizer");
   if(options){char *line=options;while(*line){char *next=strchr(line,'\n');if(next)*next++=0;char *tab=strchr(line,'\t');if(tab){
    *tab++=0;WCHAR *title=wide(tab);if(title){LRESULT i=SendDlgItemMessageW(d,251,CB_ADDSTRING,0,(LPARAM)title);if(i>=0)SendDlgItemMessageW(d,251,CB_SETITEMDATA,i,(LPARAM)_strdup(line));free(title);}
   }if(!next)break;line=next;}ap_terminal_free(options);}
   SendDlgItemMessageW(d,251,CB_SETCURSEL,0,0);
  }
  SetDlgItemTextW(d,effects?252:241,effects?brightness:syncAudio);SetDlgItemTextW(d,effects?253:242,effects?contrast:syncSubtitle);return TRUE;
 }
 if(m==WM_COMMAND&&LOWORD(w)==IDOK){
  WCHAR a[48],b[48];if(!readNumber(d,effects?252:241,a,48,effects?0:-3600,effects?2:3600)||
   !readNumber(d,effects?253:242,b,48,effects?0:-3600,effects?2:3600))return TRUE;
  if(effects){int i=SendDlgItemMessageW(d,251,CB_GETCURSEL,0,0);if(i>0){const char *v=(const char*)SendDlgItemMessageW(d,251,CB_GETITEMDATA,i,0);if(v)engineAction("equalizer",v,L"Equalizer",false);}}
  if(engineWide(effects?"brightness":"audio-delay",a,effects?L"Brightness":L"Audio delay"))wcscpy(effects?brightness:syncAudio,a);
  if(engineWide(effects?"contrast":"subtitle-delay",b,effects?L"Contrast":L"Subtitle delay"))wcscpy(effects?contrast:syncSubtitle,b);
  EndDialog(d,IDOK);return TRUE;
 }
 if(m==WM_DESTROY&&effects){int n=SendDlgItemMessageW(d,251,CB_GETCOUNT,0,0);for(int i=1;i<n;i++)free((void*)SendDlgItemMessageW(d,251,CB_GETITEMDATA,i,0));}
 if(m==WM_COMMAND&&LOWORD(w)==IDCANCEL){EndDialog(d,0);return TRUE;}return FALSE;
}
