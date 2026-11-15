enum {
 M_MULTI=900,M_FOLDER,M_DISC,M_CAPTURE,M_CLIPBOARD,M_CONVERT,M_STREAM_OUT,M_QUIT_END,M_QUIT,
 M_FWD,M_BACK,M_TIME,M_RECORD,M_VOL_UP,M_VOL_DOWN,M_MUTE,M_FIT,M_WALL,M_SNAPSHOT,M_SUBFILE,
 M_EFFECTS,M_SYNC,M_INFO,M_CODEC,M_VLM,M_GUIDE,M_MESSAGES,M_PLUGINS,M_CUSTOM,M_PREFS,
 M_QUEUE,M_DOCK,M_TOP,M_MINIMAL,M_ADVANCED,M_STATUS,M_INTERFACE,M_VLSUB,M_HELP,M_ABOUT,M_END_OUTPUT
};
extern char *ap_menu_action(Player *,const char *,const char *);
extern char *ap_menu_options(Player *,const char *);
static bool minimalUI,advancedUI,statusUI,topUI,dockUI=true,playlistVisible,quitAtEnd,wallpaperMode,recording;
static HWND queueWindow,queueList,statusWindow,pausedMark;
static size_t lastQueueSize=(size_t)-1;
static int lastQueueIndex=-2;
static HWND advancedButtons[4];
static const int advancedIds[]={M_RECORD,M_SNAPSHOT,M_BACK,M_FWD};
static HMENU extraTerminalMenu;
static void layout(HWND,int,int),start(size_t);
static WCHAR *wide(const char *);
static void relayoutPlayer(void){
 RECT rect;GetClientRect(mainwin,&rect);layout(mainwin,rect.right,rect.bottom);
 RedrawWindow(mainwin,NULL,NULL,RDW_ERASE|RDW_INVALIDATE|RDW_ALLCHILDREN);
}
static void syncQueueWindow(void){
 if(!queueList||!playlistVisible)return;
 size_t n=ap_count(player);int selected=(int)ap_index(player);
 if(n!=lastQueueSize){
  SendMessageW(queueList,WM_SETREDRAW,FALSE,0);SendMessageW(queueList,LB_RESETCONTENT,0,0);
  for(size_t i=0;i<n;i++){const char *s=ap_item(player,i);if(!s)continue;
   const char *title=s;for(const char *a=s;*a;a++)if(*a=='/'||*a=='\\')title=a+1;
   WCHAR *label=wide(*title?title:s);if(label){SendMessageW(queueList,LB_ADDSTRING,0,(LPARAM)label);free(label);}
  }
  SendMessageW(queueList,WM_SETREDRAW,TRUE,0);InvalidateRect(queueList,NULL,TRUE);lastQueueSize=n;lastQueueIndex=-2;
 }
 if(selected!=lastQueueIndex){SendMessageW(queueList,LB_SETCURSEL,selected,0);lastQueueIndex=selected;}
}
static LRESULT CALLBACK playlistProc(HWND h,UINT m,WPARAM w,LPARAM l){
 switch(m){
 case WM_CREATE:
  queueList=CreateWindowExW(WS_EX_CLIENTEDGE,L"LISTBOX",L"Playback queue",WS_CHILD|WS_VISIBLE|WS_TABSTOP|WS_VSCROLL|LBS_NOTIFY|LBS_NOINTEGRALHEIGHT,
   0,0,1,1,h,(HMENU)(INT_PTR)721,GetModuleHandleW(NULL),NULL);
  SendMessageW(queueList,WM_SETFONT,(WPARAM)uiFont,TRUE);return 0;
 case WM_SIZE:MoveWindow(queueList,0,0,LOWORD(l),HIWORD(l),TRUE);return 0;
 case WM_COMMAND:
  if(HIWORD(w)==LBN_DBLCLK){LRESULT index=SendMessageW(queueList,LB_GETCURSEL,0,0);if(index!=LB_ERR)start((size_t)index);}return 0;
 case WM_CLOSE:playlistVisible=false;ShowWindow(h,SW_HIDE);relayoutPlayer();return 0;
 }return DefWindowProcW(h,m,w,l);
}
static void recreateQueueWindow(void){
 if(queueWindow){DestroyWindow(queueWindow);queueWindow=NULL;queueList=NULL;}
 if(!playlistVisible)return;
 DWORD style=dockUI?WS_CHILD|WS_BORDER:WS_OVERLAPPEDWINDOW;
 queueWindow=CreateWindowExW(dockUI?0:WS_EX_TOOLWINDOW,L"RMPPlaylist",L"RMP Playlist",style,
  dockUI?0:CW_USEDEFAULT,dockUI?0:CW_USEDEFAULT,scale(270),scale(340),mainwin,NULL,GetModuleHandleW(NULL),NULL);
 lastQueueSize=(size_t)-1;syncQueueWindow();
 if(!dockUI)ShowWindow(queueWindow,SW_SHOWNOACTIVATE);
}
static void layoutViewExtras(int *x,int *y){
 bool normal=!fullscreen&&!minimalUI&&skin!=LINUX_THEME;
 if(queueWindow){
  bool visible=playlistVisible&&normal;ShowWindow(queueWindow,visible?SW_SHOWNA:SW_HIDE);
  if(visible&&dockUI){int width=min(scale(234),*x/3);*x-=width;MoveWindow(queueWindow,*x,0,width,*y,TRUE);}
 }
 bool status=statusUI&&normal;
 if(statusWindow){ShowWindow(statusWindow,status?SW_SHOWNA:SW_HIDE);if(status){*y-=scale(23);MoveWindow(statusWindow,0,*y,*x,scale(23),TRUE);}}
 if(advancedUI&&normal)*y-=scale(31);
 for(int i=0;i<4;i++)if(advancedButtons[i]){
  ShowWindow(advancedButtons[i],advancedUI&&normal?SW_SHOWNA:SW_HIDE);
  if(advancedUI&&normal)MoveWindow(advancedButtons[i],scale(8+i*96),*y+scale(2),scale(92),scale(27),TRUE);
 }
}
static BOOL CALLBACK findWallpaperLayer(HWND top,LPARAM result){
 if(FindWindowExW(top,NULL,L"SHELLDLL_DefView",NULL)){
  HWND worker=FindWindowExW(NULL,top,L"WorkerW",NULL);
  if(worker){*(HWND*)result=worker;return FALSE;}
 }
 return TRUE;
}
static bool setVideoWallpaper(bool active){
 if(active==wallpaperMode)return true;
 HWND parent=mainwin;
 if(active){
  HWND explorer=FindWindowW(L"Progman",NULL);if(!explorer)return false;
  DWORD_PTR ignored;SendMessageTimeoutW(explorer,0x052c,0,0,SMTO_ABORTIFHUNG,500,&ignored);
  HWND worker=NULL;EnumWindows(findWallpaperLayer,(LPARAM)&worker);
  if(!worker)worker=FindWindowExW(explorer,NULL,L"WorkerW",NULL);
  if(!worker)return false;parent=worker;
 }
 SetLastError(0);HWND previous=SetParent(video,parent);
 if(!previous && GetLastError())return false;
 wallpaperMode=active;
 if(active){RECT r;GetClientRect(parent,&r);SetWindowPos(video,HWND_BOTTOM,0,0,r.right,r.bottom,SWP_SHOWWINDOW|SWP_NOACTIVATE);}
 else relayoutPlayer();
 return true;
}
static void syncPauseOverlay(void){
 bool visible=player&&ap_state(player)==4&&IsWindowVisible(video)&&!IsIconic(mainwin)&&!wallpaperMode;
 if(!visible){if(pausedMark)ShowWindow(pausedMark,SW_HIDE);return;}
 RECT r;GetClientRect(video,&r);POINT origin={r.left,r.top};ClientToScreen(video,&origin);
 int side=min(scale(112),min(r.right,r.bottom)/2);if(side<8){if(pausedMark)ShowWindow(pausedMark,SW_HIDE);return;}
 POINT pos={origin.x+(r.right-side)/2,origin.y+(r.bottom-side)/2};
 static int lastSide,lastIcon;int icon=skin==VLC_THEME?2:1;
 if(!pausedMark)pausedMark=CreateWindowExW(WS_EX_LAYERED|WS_EX_TRANSPARENT|WS_EX_NOACTIVATE|WS_EX_TOOLWINDOW,
  L"RMPPauseOverlay",L"",WS_POPUP,0,0,side,side,mainwin,NULL,GetModuleHandleW(NULL),NULL);
 if(!pausedMark)return;
 if(side!=lastSide||icon!=lastIcon){
  BITMAPINFO bi={0};bi.bmiHeader.biSize=sizeof(bi.bmiHeader);bi.bmiHeader.biWidth=side;bi.bmiHeader.biHeight=-side;
  bi.bmiHeader.biPlanes=1;bi.bmiHeader.biBitCount=32;void *pixels=NULL;
  HDC dc=CreateCompatibleDC(NULL);HBITMAP bitmap=CreateDIBSection(dc,&bi,DIB_RGB_COLORS,&pixels,NULL,0);
  if(dc&&bitmap){
   HGDIOBJ old=SelectObject(dc,bitmap);memset(pixels,0,(size_t)side*side*4);
   HICON artwork=LoadImageW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(icon),IMAGE_ICON,side,side,0);
   if(artwork){
    DrawIconEx(dc,0,0,artwork,side,side,0,NULL,DI_NORMAL);POINT src={0};SIZE size={side,side};BLENDFUNCTION alpha={AC_SRC_OVER,0,255,AC_SRC_ALPHA};
    if(UpdateLayeredWindow(pausedMark,NULL,&pos,&size,dc,&src,0,&alpha,ULW_ALPHA)){lastSide=side;lastIcon=icon;}
    DestroyIcon(artwork);
   }
   SelectObject(dc,old);
  }
  if(bitmap)DeleteObject(bitmap);if(dc)DeleteDC(dc);
 }
 SetWindowPos(pausedMark,NULL,pos.x,pos.y,side,side,SWP_NOZORDER|SWP_NOACTIVATE|SWP_SHOWWINDOW);
}
