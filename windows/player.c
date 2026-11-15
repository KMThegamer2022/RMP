#define _UNICODE
#define COBJMACROS
#include <windows.h>
#include <commdlg.h>
#include <commctrl.h>
#include <shellapi.h>
#include <shobjidl.h>
#include <shlobj.h>
#include <math.h>
#include <propvarutil.h>
#include <initguid.h>
#include <propkey.h>
#include <dwmapi.h>
#include <uxtheme.h>
#include <vssym32.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <wchar.h>

typedef struct Player Player;
extern Player *ap_create(void *);
extern void ap_destroy(Player *), ap_add(Player *,const char *,bool), ap_toggle(Player *), ap_seek(Player *,int64_t), ap_volume(Player *,int);
extern bool ap_play(Player *,size_t),ap_seekable(Player *),ap_select(Player *,bool,int);
extern intptr_t ap_index(Player *);
extern size_t ap_count(Player *);
extern const char *ap_item(Player *,size_t);
extern int ap_state(Player *);
extern int64_t ap_time(Player *),ap_length(Player *);
extern void ap_tracks(Player *,bool,void *,void(*)(void *,int,const char *,bool));
extern char *ap_menu_options(Player *,const char *);
extern char *ap_menu_action(Player *,const char *,const char *);
extern size_t ap_library_count(Player *);
extern const char *ap_library_item(Player *,size_t);
static HMENU submenu(HMENU,const wchar_t *);
static void enqueue(const wchar_t *,bool);

enum {OPEN=101,PLAY,PREV,NEXT,FULL,STREAM,SEEK,VOLUME,STOP,LIST,REPEAT,SHUFFLE};
static HWND mainwin,video,play,position,level,clocktext;
static HMENU queue,audio,subs,bar;
static Player *player;
static WINDOWPLACEMENT placement={sizeof(WINDOWPLACEMENT)};
static bool fullscreen,dragging,smoke,repeatOne,shuffle;
static int volumePercent=80;
static ULONGLONG deadline;
static int exitcode;
static int laststate=-1;
static HFONT uiFont;
static HWND volumeLabel, idleLogo;
static HICON brandIcon;
static bool glass;
extern void ap_prepare(Player *);
extern void ap_stop(Player *);
static HICON coneIcon;
static int scale(int value){
 HDC dc=GetDC(mainwin); int dpi=GetDeviceCaps(dc,LOGPIXELSY); ReleaseDC(mainwin,dc);
 return MulDiv(value,dpi,96);
}
#include "theme.h"
#include "backdrop.h"
#include "fullscreen.h"
#include "glass-controls.h"
#include "seven-transport.h"
extern bool ap_has_video(Player *);
extern size_t ap_library_count(Player *);
extern const char *ap_library_item(Player *,size_t);
extern bool ap_library_play(Player *,size_t),ap_playlist_save(Player *,const char *),ap_playlist_load(Player *,const char *);
static wchar_t *wide(const char *);
static void start(size_t),showCurrentTrack(void),layout(HWND,int,int);
#include "media-library.h"
#include "transport-labels.h"
#include "terminal.h"
#include "menu-view.h"
#include "player-menus.h"
#include "menu-files.h"
#include "menu-commands.h"
/* Older Windows versions route wheel input to the focus owner. Follow the
   pointer over the volume control even when a child such as the queue has focus. */
static bool routeWheel(HWND target,WPARAM direction,LPARAM location){
 if(!level||!IsWindowEnabled(mainwin))return false;
 HWND root=GetAncestor(target,GA_ROOT);
 if(root!=mainwin&&root!=transport)return false;
 POINT point={(short)LOWORD(location),(short)HIWORD(location)};
 HWND hit=WindowFromPoint(point);
 if(hit==level && target!=level){
  SendMessageW(level,WM_MOUSEWHEEL,direction,location);return true;
 }
 if(target==level && hit==mediaRows){
  SendMessageW(mediaRows,WM_MOUSEWHEEL,direction,location);return true;
 }
 return false;
}
static void layout(HWND w,int x,int y){
 bool classic=skin==VLC_THEME;
 int top=!fullscreen&&skin==XP?scale(27):0, frame=!fullscreen&&skin==XP?scale(4):0;
 int height=skin==LINUX_THEME?0:scale(classic?68:mediaTheme()?MEDIA_TRANSPORT_HEIGHT:STANDARD_TRANSPORT_HEIGHT), deck=y-height;
 MoveWindow(video,frame,top,x-2*frame,max(1,(fullscreen?y:deck)-top),TRUE);
 int logo=scale(classic?120:96);
 MoveWindow(idleLogo,(x-logo)/2,((fullscreen?y:deck)-top-logo)/2,logo,logo,TRUE);
 if(mediaRows){
  layoutMediaBrowser(x,deck);
  if(fullscreen||!mediaTheme())ShowWindow(video,SW_SHOWNA);
 }
 HWND host=fullscreen?transport:w;
 if(fullscreen){int width=min(x-scale(20),scale(740));POINT origin={(x-width)/2,y-height-scale(24)};ClientToScreen(w,&origin);
 SetWindowPos(host,HWND_TOP,origin.x,origin.y,width,height,SWP_NOACTIVATE);x=width;y=height;deck=0;}
 HWND aux[]={position,level,volumeLabel,clocktext};
 for(int i=0;i<4;i++)ShowWindow(aux[i],skin==LINUX_THEME?SW_HIDE:SW_SHOWNA);
 if(skin==LINUX_THEME){
  const int ids[]={PLAY,PREV,STOP,NEXT,FULL,LIST,REPEAT,SHUFFLE,OPEN};
  for(size_t i=0;i<sizeof(ids)/sizeof(*ids);i++)ShowWindow(GetDlgItem(host,ids[i]),SW_HIDE);
  ShowWindow(idleLogo,SW_HIDE);ShowWindow(transport,SW_HIDE);return;
 }
 if(mediaTheme()){layoutClassicTransport(host,x,deck);return;}
 int cy=y-scale(classic?34:48);
 MoveWindow(position,scale(10),deck+scale(3),x-scale(142),scale(24),TRUE);
 MoveWindow(clocktext,x-scale(129),deck+scale(2),scale(125),scale(24),TRUE);
 int ids[]={PLAY,PREV,STOP,NEXT,FULL,LIST,REPEAT,SHUFFLE,OPEN};
 int left=scale(8);
 for(unsigned i=0;i<sizeof(ids)/sizeof(ids[0]);i++){
 HWND b=GetDlgItem(host,ids[i]);bool extra=ids[i]==LIST||ids[i]==REPEAT||ids[i]==SHUFFLE;
 bool show=classic?(ids[i]!=OPEN):!extra;
 ShowWindow(b,show?SW_SHOWNA:SW_HIDE);if(!show)continue;
 int width=scale(classic?(i==0?40:32):(i==2?48:i==4?64:76));
 MoveWindow(b,left,cy,width,scale(classic?29:36),TRUE);left+=width+scale(classic?2:4);
 }
 MoveWindow(volumeLabel,x-scale(155),cy,scale(48),scale(29),TRUE);MoveWindow(level,x-scale(103),cy+scale(2),scale(96),scale(24),TRUE);

}
static void applyTheme(HWND w,int value){
 bool changing=value!=skin;
 skin=value;
 if(changing){awaitingVideo=false;if(mediaTheme())browserPage=skin==XP?NAV_NOW:NAV_LIBRARY;}
 if(panelBrush)DeleteObject(panelBrush);panelBrush=CreateSolidBrush(skin==SEVEN?GetSysColor(COLOR_BTNFACE):color(skin==XP?0xc8d5e8:themes[skin].panel));
 if(changing){
  if(skin==LINUX_THEME)PostMessageW(w,REVEAL_TERMINAL,0,0);
  else{
   if(terminalWindow)ShowWindow(terminalWindow,SW_HIDE);
   if(player&&ap_index(player)<0)ShowWindow(idleLogo,SW_SHOWNA);
  }
 }
 EnableMenuItem(bar,SHOW_TERMINAL,MF_BYCOMMAND|(skin==LINUX_THEME?MF_ENABLED:MF_GRAYED));
 CheckMenuRadioItem(themeMenu,THEME_FIRST,THEME_FIRST+THEME_COUNT-1,THEME_FIRST+skin,MF_BYCOMMAND);
 updateGlass(w);
 HWND host=fullscreen?transport:w;
 int ids[]={PLAY,NEXT,PREV,STOP,FULL,OPEN,LIST,REPEAT,SHUFFLE};for(unsigned i=0;i<sizeof(ids)/sizeof(ids[0]);i++){
 HWND button=GetDlgItem(host,ids[i]);SendMessageW(button,BM_SETSTYLE,skin==SEVEN && contrastMode?BS_PUSHBUTTON:BS_OWNERDRAW,TRUE);}
 SendMessageW(idleLogo,STM_SETICON,(WPARAM)(skin==VLC_THEME?coneIcon:brandIcon),0);
 /* Windows 11 honors caption color. Older versions keep their native caption. */
 COLORREF caption=contrastMode?0xffffffff:color(skin==XP?0x245edb:skin==MODERN?0xf5f5f5:themes[skin].panel);
 if(skin==SEVEN)caption=0xffffffff;
 DwmSetWindowAttribute(w,35,&caption,sizeof(caption));
 COLORREF captionInk=contrastMode?0xffffffff:color(skin==XP||skin==MAC||skin==RMP_DEFAULT?0xffffff:0x152334);
 if(skin==LINUX_THEME)captionInk=color(0xdbece0);
 if(skin==SEVEN)captionInk=0xffffffff;
 DwmSetWindowAttribute(w,36,&captionInk,sizeof(captionInk));
 int corners=skin==SEVEN?0:(skin==MODERN||skin==XP?1:2);
 DwmSetWindowAttribute(w,33,&corners,sizeof(corners));
 HFONT nextFont=CreateFontW(-scale(skin==XP?13:14),0,0,0,FW_NORMAL,FALSE,FALSE,FALSE,DEFAULT_CHARSET,0,0,CLEARTYPE_QUALITY,0,skin==XP?L"Tahoma":L"Segoe UI");
 if(nextFont){HFONT old=uiFont;uiFont=nextFont;
 for(HWND child=GetWindow(host,GW_CHILD);child;child=GetWindow(child,GW_HWNDNEXT))SendMessageW(child,WM_SETFONT,(WPARAM)uiFont,FALSE);
 if(host!=w)for(HWND child=GetWindow(w,GW_CHILD);child;child=GetWindow(child,GW_HWNDNEXT))SendMessageW(child,WM_SETFONT,(WPARAM)uiFont,FALSE);
 if(old)DeleteObject(old);}

 styleMediaRows();
 RECT r;GetClientRect(w,&r);layout(w,r.right,r.bottom);
 rebuildMediaRows();
 RedrawWindow(w,NULL,NULL,RDW_INVALIDATE|RDW_FRAME|RDW_ERASE|RDW_ALLCHILDREN);
 InvalidateRect(level,NULL,TRUE);InvalidateRect(position,NULL,TRUE);
}
static wchar_t *wide(const char *s){
 int n=MultiByteToWideChar(CP_UTF8,0,s,-1,NULL,0);
 wchar_t *out=calloc(n,sizeof(wchar_t)); if(out)MultiByteToWideChar(CP_UTF8,0,s,-1,out,n);return out;
}
static void enqueue(const wchar_t *s,bool url){
 int n=WideCharToMultiByte(CP_UTF8,0,s,-1,NULL,0,NULL,NULL);
 char *out=malloc(n);if(out){WideCharToMultiByte(CP_UTF8,0,s,-1,out,n,NULL,NULL);ap_add(player,out,url);free(out);}
}
static const char *basename8(const char *s){const char *p=s;for(;*s;s++)if(*s=='/'||*s=='\\')p=s+1;return p;}
static void start(size_t index){
 if(index>=ap_count(player))return;
 if(!ap_play(player,index)){MessageBoxW(mainwin,L"Could not open this media.",L"RMP",MB_ICONERROR);return;}
 showCurrentTrack();
}
static void showCurrentTrack(void){
 intptr_t index=ap_index(player);
 if(index<0)return;
 // LibVLC discovers the streams asynchronously. Keep the library available
 // for audio and reveal the picture once this playback has a video output.
 awaitingVideo=mediaTheme()&&browserPage!=NAV_NOW;
 if(mediaTheme())rebuildMediaRows();
 ShowWindow(idleLogo,SW_HIDE);
 wchar_t *title=wide(basename8(ap_item(player,(size_t)index)));SetWindowTextW(mainwin,title);free(title);laststate=-1;
}
static void openfile(void){
 ap_prepare(player);
 wchar_t path[32768]={0};OPENFILENAMEW f={sizeof(f)};f.hwndOwner=mainwin;f.lpstrFilter=L"Media files\0*.*\0";f.lpstrFile=path;f.nMaxFile=32768;f.Flags=OFN_FILEMUSTEXIST|OFN_NOCHANGEDIR;
 if(GetOpenFileNameW(&f)){enqueue(path,false);start(ap_count(player)-1);}
}
static INT_PTR CALLBACK streamdlg(HWND d,UINT m,WPARAM w,LPARAM l){
 if(m==WM_COMMAND && LOWORD(w)==IDOK){wchar_t url[8192];GetDlgItemTextW(d,201,url,8192);
 if(!wcsstr(url,L"://")){MessageBoxW(d,L"Enter a URL including its protocol.",L"RMP",MB_OK);return TRUE;}
 enqueue(url,true);start(ap_count(player)-1);EndDialog(d,1);return TRUE;}
 if(m==WM_COMMAND && LOWORD(w)==IDCANCEL){EndDialog(d,0);return TRUE;}return FALSE;
}
static void togglefull(void){
 if(!fullscreen){moveTransport(mainwin,transport);fullscreen=true;GetCursorPos(&lastMouse);hideAt=GetTickCount64()+2500;GetWindowPlacement(mainwin,&placement);MONITORINFO mi={sizeof(mi)};GetMonitorInfoW(MonitorFromWindow(mainwin,MONITOR_DEFAULTTONEAREST),&mi);
 SetMenu(mainwin,NULL);SetWindowLongPtrW(mainwin,GWL_STYLE,WS_POPUP|WS_VISIBLE|WS_CLIPCHILDREN);
 SetWindowPos(mainwin,HWND_TOP,mi.rcMonitor.left,mi.rcMonitor.top,mi.rcMonitor.right-mi.rcMonitor.left,mi.rcMonitor.bottom-mi.rcMonitor.top,SWP_FRAMECHANGED);}
 else{ShowWindow(transport,SW_HIDE);moveTransport(transport,mainwin);fullscreen=false;SetWindowLongPtrW(mainwin,GWL_STYLE,WS_OVERLAPPEDWINDOW|WS_VISIBLE|WS_CLIPCHILDREN);SetMenu(mainwin,bar);SetWindowPlacement(mainwin,&placement);SetWindowPos(mainwin,NULL,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_FRAMECHANGED);}
 applyTheme(mainwin,skin);if(fullscreen&&skin!=LINUX_THEME)ShowWindow(transport,SW_SHOWNOACTIVATE);
}
static LRESULT CALLBACK videoProc(HWND h,UINT msg,WPARAM wp,LPARAM lp,UINT_PTR id,DWORD_PTR data){
 if(msg==WM_LBUTTONDBLCLK){PostMessageW(mainwin,WM_COMMAND,FULL,0);return 0;}
 if(msg==WM_CTLCOLORSTATIC){SetBkColor((HDC)wp,RGB(0,0,0));return (LRESULT)GetStockObject(BLACK_BRUSH);}
 if(msg==WM_PAINT){PAINTSTRUCT ps;HDC dc=BeginPaint(h,&ps);FillRect(dc,&ps.rcPaint,GetStockObject(BLACK_BRUSH));EndPaint(h,&ps);return 0;}
 return DefSubclassProc(h,msg,wp,lp);}
static HWND control(HWND w,const wchar_t *cls,const wchar_t *title,int id,DWORD style){
 HWND h=CreateWindowW(cls,title,WS_CHILD|WS_VISIBLE|style,0,0,10,10,w,(HMENU)(intptr_t)id,GetModuleHandleW(NULL),NULL);SendMessageW(h,WM_SETFONT,(WPARAM)uiFont,TRUE);
 if(w==mainwin && !(style&WS_CLIPCHILDREN))SetWindowSubclass(h,glassControlProc,2,0);
 return h;
}
static void nextItem(void){size_t n=ap_count(player);size_t i=ap_index(player)+1;if(shuffle&&n>1)i=(i+(size_t)(GetTickCount64()%(n-1)))%n;start(i);}
static void setTransportText(HWND control,const WCHAR *text){
 WCHAR old[100];GetWindowTextW(control,old,100);
 if(lstrcmpW(old,text))SetWindowTextW(control,text);
}
static void tick(void){
 updateTransport();
 if(mediaTheme()){markPlayingInQueue();requestVisibleDetails();beginTagSearch();}
 if(skin==VLC_THEME){wchar_t percent[12];swprintf(percent,12,L"%d%%",volumePercent);setTransportText(volumeLabel,percent);}
 else setTransportText(volumeLabel,L"Volume");
 int s=ap_state(player);
 if(awaitingVideo){
  if(ap_has_video(player))showMediaPage(NAV_NOW);
  else if(s>=5)awaitingVideo=false;
 }
 if(s==6 && laststate!=6){if(repeatOne){start(ap_index(player));return;}if(shuffle||ap_index(player)+1<(intptr_t)ap_count(player)){nextItem();return;}}laststate=s;
 int64_t t=ap_time(player),d=ap_length(player);if(t<0)t=0;
 SetWindowTextW(play,s>=1&&s<=3?L"Ⅱ  Pause":L"▶  Play");EnableWindow(position,ap_seekable(player)&&d>0);
 if(!dragging)SendMessageW(position,TBM_SETPOS,TRUE,d>0?t*1000/d:0);
 wchar_t text[100];swprintf(text,100,L"%lld:%02lld / %lld:%02lld",t/60000,t/1000%60,d>0?d/60000:0,d>0?d/1000%60:0);
 setTransportText(clocktext,s==7?L"Playback error":text);
}
static void track(void *menu,int id,const char *name,bool selected){
 HMENU m=menu;wchar_t *title=wide(name);MENUITEMINFOW i={sizeof(i)};i.fMask=MIIM_STRING|MIIM_DATA|MIIM_STATE;i.dwTypeData=title;i.dwItemData=(ULONG_PTR)(intptr_t)id;i.fState=selected?MFS_CHECKED:MFS_UNCHECKED;InsertMenuItemW(m,GetMenuItemCount(m),TRUE,&i);free(title);
}
static HMENU submenu(HMENU parent,const wchar_t *name){
 HMENU m=CreatePopupMenu();AppendMenuW(parent,MF_POPUP,(UINT_PTR)m,name);MENUINFO info={sizeof(info),MIM_STYLE,MNS_NOTIFYBYPOS};SetMenuInfo(m,&info);return m;
}

#include "update.h"

static LRESULT CALLBACK windowproc(HWND w,UINT msg,WPARAM wp,LPARAM lp){
 switch(msg){

 case WM_CREATE:{
 mainwin=w;uiFont=CreateFontW(-scale(14),0,0,0,FW_NORMAL,FALSE,FALSE,FALSE,DEFAULT_CHARSET,0,0,CLEARTYPE_QUALITY,0,L"Segoe UI");updateGlass(w);createPlayerMenus(w);
 video=control(w,L"STATIC",L"",0,SS_BLACKRECT|SS_NOTIFY|WS_CLIPCHILDREN);SetWindowSubclass(video,videoProc,1,0);player=ap_create(video);if(!player)return -1;
 idleLogo=control(video,L"STATIC",L"",0,SS_ICON|SS_CENTERIMAGE|WS_DISABLED);
 brandIcon=LoadImageW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(1),IMAGE_ICON,scale(96),scale(96),0);
 SendMessageW(idleLogo,STM_SETICON,(WPARAM)brandIcon,0);
 control(w,L"BUTTON",L"Open file…",OPEN,BS_OWNERDRAW);play=control(w,L"BUTTON",L"▶  Play",PLAY,BS_OWNERDRAW);control(w,L"BUTTON",L"Previous",PREV,BS_OWNERDRAW);control(w,L"BUTTON",L"Next",NEXT,BS_OWNERDRAW);volumeLabel=control(w,L"STATIC",L"Volume",0,SS_OWNERDRAW);
 coneIcon=LoadImageW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(2),IMAGE_ICON,scale(120),scale(120),0);
 control(w,L"BUTTON",L"☷",LIST,BS_OWNERDRAW);control(w,L"BUTTON",L"↻",REPEAT,BS_OWNERDRAW);control(w,L"BUTTON",L"⤨",SHUFFLE,BS_OWNERDRAW);
 control(w,L"BUTTON",L"■",STOP,BS_OWNERDRAW);control(w,L"BUTTON",L"⛶",FULL,BS_OWNERDRAW);
 transport=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE,L"RMPTransport",L"Playback controls",WS_POPUP|WS_BORDER|WS_CLIPCHILDREN,0,0,1,1,w,NULL,GetModuleHandleW(NULL),NULL);

 position=control(w,TRACKBAR_CLASSW,L"",SEEK,TBS_NOTICKS);level=control(w,TRACKBAR_CLASSW,L"",VOLUME,TBS_NOTICKS);SetWindowTextW(level,L"Volume");clocktext=control(w,L"STATIC",L"",0,SS_OWNERDRAW);
 makeMediaBrowser();
 SendMessageW(position,TBM_SETRANGE,TRUE,MAKELONG(0,1000));SendMessageW(level,TBM_SETPOS,TRUE,80);SetTimer(w,1,200,NULL);DragAcceptFiles(w,TRUE);applyTheme(w,savedTheme());return 0;}

 case REVEAL_TERMINAL:revealTerminal();return 0;
 case WM_MOUSEWHEEL:if(routeWheel(w,wp,lp))return 0;break;
 case WM_GETMINMAXINFO:((MINMAXINFO*)lp)->ptMinTrackSize=(POINT){scale(720),scale(340)};return 0;
 case WM_SIZE:if(video)layout(w,LOWORD(lp),HIWORD(lp));return 0;
 case WM_DWMCOMPOSITIONCHANGED:case WM_SETTINGCHANGE:case WM_THEMECHANGED:{
 if(video && themeMenu)applyTheme(w,skin);break;}
 case WM_ERASEBKGND:{
 RECT r;GetClientRect(w,&r);HDC dc=(HDC)wp;Theme t=themes[skin];
 if(contrastMode)FillRect(dc,&r,GetSysColorBrush(COLOR_BTNFACE));
 else if(mediaTheme())paintMediaSurface(dc,r);
 else fillColor(dc,r,color(t.panel));
 return 1;}
 case MEDIA_SEARCH_READY:receiveTagSearch((TagSearch*)lp);return 0;
 case MEDIA_DETAILS_READY:receiveMediaDetails((MediaDetails*)lp);return 0;
 case WM_NOTIFY:{
 NMHDR *notice=(NMHDR *)lp;
 if(notice->hwndFrom==mediaRows && notice->code==LVN_GETEMPTYMARKUP){
  NMLVEMPTYMARKUP *empty=(NMLVEMPTYMARKUP*)lp;empty->dwFlags=EMF_CENTERED;
  const WCHAR *message=GetWindowTextLengthW(mediaFilter)&&browserPage!=NAV_NOW?L"No matching tracks":
   browserPage==NAV_LIBRARY?L"Open a media file to add it to your library.":L"Your current playlist is empty.";
  lstrcpynW(empty->szMarkup,message,sizeof(empty->szMarkup)/sizeof(WCHAR));return TRUE;
 }
 if(notice->hwndFrom==mediaRows && (notice->code==NM_DBLCLK||notice->code==NM_RETURN)){
  playMediaRow();return 0;
 }
 if(!contrastMode && skin!=MAC && skin!=SEVEN && notice->code==NM_CUSTOMDRAW && (notice->hwndFrom==position||notice->hwndFrom==level)){
 NMCUSTOMDRAW *draw=(NMCUSTOMDRAW *)lp;
 if(draw->dwDrawStage==CDDS_PREPAINT && skin==XP){xpControlBackdrop(notice->hwndFrom,draw->hdc);return CDRF_NOTIFYITEMDRAW;}
 if(draw->dwDrawStage==CDDS_PREPAINT && skin==VLC_THEME && notice->hwndFrom==level){
 RECT r;GetClientRect(level,&r);fillColor(draw->hdc,r,color(themes[skin].panel));InflateRect(&r,-3,-3);
 POINT points[]={{r.left,r.bottom},{r.right,r.top},{r.right,r.bottom}};
 HRGN triangle=CreatePolygonRgn(points,3,WINDING);int save=SaveDC(draw->hdc);ExtSelectClipRgn(draw->hdc,triangle,RGN_AND);
 fillColor(draw->hdc,r,color(0xe0e0e0));RECT filled=r;filled.right=r.left+MulDiv(r.right-r.left,SendMessageW(level,TBM_GETPOS,0,0),100);fillColor(draw->hdc,filled,color(0x48c638));RestoreDC(draw->hdc,save);
 SetDCBrushColor(draw->hdc,color(0x777777));FrameRgn(draw->hdc,triangle,GetStockObject(DC_BRUSH),1,1);DeleteObject(triangle);return CDRF_SKIPDEFAULT;}
 if(draw->dwDrawStage==CDDS_PREPAINT && skin==VLC_THEME && notice->hwndFrom==position){
 RECT r;GetClientRect(position,&r);fillColor(draw->hdc,r,color(themes[skin].panel));
 RECT track=r;InflateRect(&track,-scale(5),0);int mid=(track.top+track.bottom)/2;track.top=mid-scale(3);track.bottom=mid+scale(3);
 blend(draw->hdc,track,0xa6a6a6,0xe4e4e4);if(IsWindowEnabled(position)){
 int xpos=track.left+MulDiv(track.right-track.left,SendMessageW(position,TBM_GETPOS,0,0),1000);
 RECT thumb={xpos-scale(3),mid-scale(6),xpos+scale(3),mid+scale(6)};blend(draw->hdc,thumb,0xffffff,0xa0a0a0);}
 return CDRF_SKIPDEFAULT;}
 if(draw->dwDrawStage==CDDS_PREPAINT)return CDRF_NOTIFYITEMDRAW;
 if(draw->dwDrawStage==CDDS_ITEMPREPAINT){
 RECT r=draw->rc;Theme t=themes[skin];
 if(draw->dwItemSpec==TBCD_CHANNEL){
 int mid=(r.top+r.bottom)/2;r.top=mid-scale(2);r.bottom=mid+scale(2);
 fillColor(draw->hdc,r,color(t.edge));int pos=SendMessageW(notice->hwndFrom,TBM_GETPOS,0,0);
 int limit=SendMessageW(notice->hwndFrom,TBM_GETRANGEMAX,0,0);
 if(limit>0){r.right=r.left+MulDiv(r.right-r.left,pos,limit);fillColor(draw->hdc,r,color(t.accent));}
 return CDRF_SKIPDEFAULT;}
 if(draw->dwItemSpec==TBCD_THUMB){
 if(skin==XP){fillColor(draw->hdc,r,color(t.face));DrawEdge(draw->hdc,&r,EDGE_RAISED,BF_RECT);RECT grip=r;InflateRect(&grip,-2,-2);grip.bottom=grip.top+scale(3);fillColor(draw->hdc,grip,color(0x60a437));}
 else{fillColor(draw->hdc,r,color(t.accent));}
 return CDRF_SKIPDEFAULT;}
 }}
 break;}
 case WM_DRAWITEM:
 if(((DRAWITEMSTRUCT*)lp)->CtlType==ODT_STATIC){paintTransportLabel((DRAWITEMSTRUCT*)lp);return TRUE;}
 if(((DRAWITEMSTRUCT*)lp)->CtlType==ODT_BUTTON){
 int id=((DRAWITEMSTRUCT*)lp)->CtlID;
 if(id>=NAV_NOW && id<=ADD_FILE)paintBrowserButton((DRAWITEMSTRUCT*)lp);
 else if(skin==XP && !contrastMode)paintXpTransport((DRAWITEMSTRUCT*)lp);
 else if(skin==SEVEN && !contrastMode)paintSevenFace((DRAWITEMSTRUCT*)lp);else paintButton((DRAWITEMSTRUCT*)lp);return TRUE;}break;
 case WM_CTLCOLORSTATIC:
 if((HWND)lp==video || (HWND)lp==idleLogo){SetBkColor((HDC)wp,RGB(0,0,0));return (LRESULT)GetStockObject(BLACK_BRUSH);}
 if(skin==SEVEN)break;
 if((HWND)lp!=video && (HWND)lp!=idleLogo && panelBrush && !contrastMode){
 SetBkColor((HDC)wp,color(themes[skin].panel));SetTextColor((HDC)wp,color(themes[skin].text));return (LRESULT)panelBrush;}break;
 case WM_INITMENUPOPUP:{HMENU m=(HMENU)wp;
 if(m==queue){while(GetMenuItemCount(m)>0)DeleteMenu(m,0,MF_BYPOSITION);for(size_t i=0;i<ap_count(player);i++)track(m,(int)i,basename8(ap_item(player,i)),i==(size_t)ap_index(player));return 0;}
 if(fillPlayerMenu(m))return 0;
 break;}
 case WM_MENUCOMMAND:{HMENU m=(HMENU)lp;MENUITEMINFOW item={sizeof(item)};item.fMask=MIIM_DATA;GetMenuItemInfoW(m,wp,TRUE,&item);if(m==queue)start(item.dwItemData);else selectMenuOption(m,(UINT)wp);return 0;}
 case WM_COMMAND:
 if(menuCommand(w,LOWORD(wp)))return 0;
 switch(LOWORD(wp)){
 case NAV_NOW:case NAV_LIBRARY:case NAV_QUEUE:showMediaPage(LOWORD(wp));return 0;
 case FILTER:if(HIWORD(wp)==EN_CHANGE)rebuildMediaRows();return 0;
 case LOAD_LIST:case SAVE_LIST:playlistFile(LOWORD(wp)==SAVE_LIST);return 0;
 case ADD_FILE:openfile();return 0;
 case SHOW_TERMINAL:revealTerminal();return 0;
 }
 if(LOWORD(wp)>=THEME_FIRST && LOWORD(wp)<THEME_FIRST+THEME_COUNT){int value=LOWORD(wp)-THEME_FIRST;if(value==LINUX_THEME && skin!=LINUX_THEME)storeTheme(skin);if(value==LINUX_THEME&&skin==value)PostMessageW(w,REVEAL_TERMINAL,0,0);applyTheme(w,value);storeTheme(value);return 0;}
 switch(LOWORD(wp)){case 490:if(!launchCheck(true))MessageBoxW(w,L"The update helper could not start. Reinstall RMP to restore it.",L"RMP update",MB_ICONERROR);break;case LIST:{RECT r;GetWindowRect(GetDlgItem(fullscreen?transport:w,LIST),&r);TrackPopupMenu(queue,TPM_LEFTALIGN|TPM_BOTTOMALIGN,r.left,r.top,0,w,NULL);break;}
 case REPEAT:repeatOne=!repeatOne;SetWindowTextW((HWND)lp,repeatOne?L"↻¹":L"↻");break;
 case SHUFFLE:shuffle=!shuffle;SetWindowTextW((HWND)lp,shuffle?L"⤨✓":L"⤨");break;
 case STOP:ap_stop(player);break;case OPEN:openfile();break;case PLAY:if(ap_count(player))ap_toggle(player);else openfile();break;case NEXT:nextItem();break;case PREV:start(ap_index(player)>0?ap_index(player)-1:0);break;case FULL:togglefull();break;case STREAM:DialogBoxW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(200),w,streamdlg);break;}return 0;
 case WM_HSCROLL:if((HWND)lp==level){volumePercent=SendMessageW(level,TBM_GETPOS,0,0);ap_volume(player,volumePercent);InvalidateRect(level,NULL,TRUE);}else if((HWND)lp==position){dragging=LOWORD(wp)==TB_THUMBTRACK;if(!dragging)ap_seek(player,ap_length(player)*SendMessageW(position,TBM_GETPOS,0,0)/1000);}return 0;
 case WM_TIMER:
 tick();
 if(smoke && ((ap_state(player)==3 && ap_time(player)>100) || GetTickCount64()>deadline)){
 exitcode=ap_state(player)==3 && ap_time(player)>100?0:2;DestroyWindow(w);
 }return 0;
 case WM_DROPFILES:{HDROP drop=(HDROP)wp;size_t first=ap_count(player);UINT n=DragQueryFileW(drop,0xffffffff,NULL,0);for(UINT i=0;i<n;i++){wchar_t path[32768];DragQueryFileW(drop,i,path,32768);enqueue(path,false);}DragFinish(drop);start(first);return 0;}
 case WM_DESTROY:if(terminalWindow)DestroyWindow(terminalWindow);InterlockedIncrement(&searchEpoch);clearDetails();for(int i=0;i<9;i++)if(sevenIcons[i])DestroyIcon(sevenIcons[i]);KillTimer(w,1);DestroyIcon(coneIcon);if(player)ap_destroy(player);DeleteObject(panelBrush);DestroyIcon(brandIcon);DeleteObject(uiFont);PostQuitMessage(exitcode);return 0;
 }
 return DefWindowProcW(w,msg,wp,lp);
}

int WINAPI wWinMain(HINSTANCE h,HINSTANCE prev,LPWSTR cmd,int show){
 BufferedPaintInit();SetProcessDPIAware();INITCOMMONCONTROLSEX controls={sizeof(controls),ICC_BAR_CLASSES|ICC_LISTVIEW_CLASSES};InitCommonControlsEx(&controls);
 WNDCLASSW c={0};c.lpfnWndProc=windowproc;c.hInstance=h;c.lpszClassName=L"RMPPlayer";c.hIcon=LoadIconW(h,MAKEINTRESOURCEW(1));c.hCursor=LoadCursor(NULL,IDC_ARROW);c.hbrBackground=(HBRUSH)(COLOR_BTNFACE+1);RegisterClassW(&c);
 WNDCLASSW floating={0};floating.lpfnWndProc=transportProc;floating.hInstance=h;floating.lpszClassName=L"RMPTransport";floating.hCursor=c.hCursor;RegisterClassW(&floating);
 WNDCLASSW terminalClass={0};terminalClass.lpfnWndProc=terminalProc;terminalClass.lpszClassName=L"RMPTerminal";terminalClass.hInstance=h;terminalClass.hCursor=c.hCursor;terminalClass.hIcon=c.hIcon;RegisterClassW(&terminalClass);
 HWND w=CreateWindowW(c.lpszClassName,L"RMP · Rust Media Player",WS_OVERLAPPEDWINDOW|WS_CLIPCHILDREN,CW_USEDEFAULT,CW_USEDEFAULT,900,570,NULL,NULL,h,NULL);if(!w)return 1;

 int argc;wchar_t **argv=CommandLineToArgvW(GetCommandLineW(),&argc);smoke=argc==3 && wcscmp(argv[1],L"--smoke-test")==0;deadline=GetTickCount64()+20000;for(int i=smoke?2:1;i<argc;i++)enqueue(argv[i],wcsstr(argv[i],L"://")!=NULL);LocalFree(argv);if(argc>1)start(0);
 ShowWindow(w,show);if(!smoke)launchCheck(false);MSG m;while(GetMessageW(&m,NULL,0,0)>0){
 if(m.message==WM_MOUSEWHEEL && routeWheel(m.hwnd,m.wParam,m.lParam))continue;
 if(m.message==WM_KEYDOWN && (m.wParam==VK_F11 || (m.wParam==VK_ESCAPE && fullscreen))){togglefull();continue;}if(menuKeys && TranslateAcceleratorW(mainwin,menuKeys,&m))continue;TranslateMessage(&m);DispatchMessageW(&m);}BufferedPaintUnInit();return (int)m.wParam;
}
