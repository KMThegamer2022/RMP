#define _UNICODE
#include <windows.h>
#include <commdlg.h>
#include <commctrl.h>
#include <shellapi.h>
#include <dwmapi.h>
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

enum {OPEN=101,PLAY,PREV,NEXT,FULL,STREAM,SEEK,VOLUME};
static HWND mainwin,video,play,position,level,clocktext;
static HMENU queue,audio,subs,bar;
static Player *player;
static WINDOWPLACEMENT placement={sizeof(WINDOWPLACEMENT)};
static bool fullscreen,dragging,smoke;
static ULONGLONG deadline;
static int exitcode;
static int laststate=-1;
static HFONT uiFont;
static HWND volumeLabel, idleLogo;
static HICON brandIcon;
static bool glass;
static int scale(int value){
 HDC dc=GetDC(mainwin); int dpi=GetDeviceCaps(dc,LOGPIXELSY); ReleaseDC(mainwin,dc);
 return MulDiv(value,dpi,96);
}
#include "theme.h"
#include "backdrop.h"
static void layout(HWND w,int x,int y){
 int top=skin==XP?scale(27):sevenFrame()?scale(56):0;
 int frame=skin==XP?scale(4):skin==SEVEN?scale(6):0;
 int deck=y-scale(100),cy=y-scale(55),mid=x/2;
 int buttons[]={CAP_MIN,CAP_MAX,CAP_CLOSE};int left=x-scale(111);
 for(int i=0;i<3;i++){HWND b=GetDlgItem(w,buttons[i]);int width=scale(i==2?46:29);if(b){ShowWindow(b,sevenFrame()?SW_SHOWNA:SW_HIDE);MoveWindow(b,left,0,width,scale(21),TRUE);}left+=width;}
 HWND menuButton=GetDlgItem(w,CAP_MENU);
 if(menuButton){ShowWindow(menuButton,sevenFrame()?SW_SHOWNA:SW_HIDE);MoveWindow(menuButton,scale(7),scale(32),scale(69),scale(23),TRUE);}
 MoveWindow(video,frame,top,x-2*frame,max(1,deck-top),TRUE);
 MoveWindow(idleLogo,(x-scale(96))/2,(deck-top-scale(96))/2,scale(96),scale(96),TRUE);
 MoveWindow(position,scale(16),deck+scale(8),x-scale(166),scale(27),TRUE);
 MoveWindow(clocktext,x-scale(147),deck+scale(6),scale(140),scale(28),TRUE);
 MoveWindow(GetDlgItem(w,OPEN),scale(16),cy,scale(87),scale(33),TRUE);
 MoveWindow(GetDlgItem(w,THEME_BUTTON),scale(109),cy,scale(78),scale(33),TRUE);
 MoveWindow(GetDlgItem(w,PREV),mid-scale(123),cy,scale(72),scale(33),TRUE);
 MoveWindow(play,mid-scale(46),cy-scale(4),scale(92),scale(41),TRUE);
 MoveWindow(GetDlgItem(w,NEXT),mid+scale(51),cy,scale(72),scale(33),TRUE);
 MoveWindow(volumeLabel,x-scale(170),cy,scale(52),scale(33),TRUE);
 MoveWindow(level,x-scale(118),cy+scale(4),scale(102),scale(26),TRUE);
}
static void applyTheme(HWND w,int value){
 skin=value;
 if(panelBrush)DeleteObject(panelBrush);panelBrush=CreateSolidBrush(color(themes[skin].panel));
 CheckMenuRadioItem(themeMenu,THEME_FIRST,THEME_FIRST+THEME_COUNT-1,THEME_FIRST+skin,MF_BYCOMMAND);
 updateGlass(w);
 if(!fullscreen){
 HMENU visibleMenu=sevenFrame()?NULL:bar;if(GetMenu(w)!=visibleMenu)SetMenu(w,visibleMenu);
 LONG_PTR style=GetWindowLongPtrW(w,GWL_STYLE);
 LONG_PTR wanted=sevenFrame()?style&~WS_CAPTION:style|WS_CAPTION;
 if(wanted!=style){SetWindowLongPtrW(w,GWL_STYLE,wanted);SetWindowPos(w,NULL,0,0,0,0,SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE);}
 }
 /* Windows 11 honors caption color. Older versions keep their native caption. */
 COLORREF caption=contrastMode?0xffffffff:color(skin==XP?0x245edb:skin==MODERN?0xf5f5f5:themes[skin].panel);
 DwmSetWindowAttribute(w,35,&caption,sizeof(caption));
 COLORREF captionInk=contrastMode?0xffffffff:color(skin==XP||skin==MAC?0xffffff:0x152334);
 DwmSetWindowAttribute(w,36,&captionInk,sizeof(captionInk));
 int corners=skin==MODERN||skin==XP?1:2;
 DwmSetWindowAttribute(w,33,&corners,sizeof(corners));
 HFONT nextFont=CreateFontW(-scale(skin==XP?13:14),0,0,0,FW_NORMAL,FALSE,FALSE,FALSE,DEFAULT_CHARSET,0,0,CLEARTYPE_QUALITY,0,skin==XP?L"Tahoma":L"Segoe UI");
 if(nextFont){HFONT old=uiFont;uiFont=nextFont;
 for(HWND child=GetWindow(w,GW_CHILD);child;child=GetWindow(child,GW_HWNDNEXT))SendMessageW(child,WM_SETFONT,(WPARAM)uiFont,FALSE);
 DeleteObject(old);}

 RECT r;GetClientRect(w,&r);layout(w,r.right,r.bottom);
 RedrawWindow(w,NULL,NULL,RDW_INVALIDATE|RDW_FRAME|RDW_ERASE|RDW_ALLCHILDREN);
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
 ShowWindow(idleLogo,SW_HIDE);
 wchar_t *title=wide(basename8(ap_item(player,index)));SetWindowTextW(mainwin,title);free(title);laststate=-1;
}
static void openfile(void){
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
 if(!fullscreen){GetWindowPlacement(mainwin,&placement);MONITORINFO mi={sizeof(mi)};GetMonitorInfoW(MonitorFromWindow(mainwin,MONITOR_DEFAULTTONEAREST),&mi);
 SetMenu(mainwin,NULL);SetWindowLongPtrW(mainwin,GWL_STYLE,WS_POPUP|WS_VISIBLE|WS_CLIPCHILDREN);
 SetWindowPos(mainwin,HWND_TOP,mi.rcMonitor.left,mi.rcMonitor.top,mi.rcMonitor.right-mi.rcMonitor.left,mi.rcMonitor.bottom-mi.rcMonitor.top,SWP_FRAMECHANGED);}
 else{SetWindowLongPtrW(mainwin,GWL_STYLE,WS_OVERLAPPEDWINDOW|WS_VISIBLE|WS_CLIPCHILDREN);SetMenu(mainwin,bar);SetWindowPlacement(mainwin,&placement);SetWindowPos(mainwin,NULL,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_FRAMECHANGED);}
 fullscreen=!fullscreen;applyTheme(mainwin,skin);
}
static LRESULT CALLBACK videoProc(HWND h,UINT msg,WPARAM wp,LPARAM lp,UINT_PTR id,DWORD_PTR data){
 if(msg==WM_CTLCOLORSTATIC){SetBkColor((HDC)wp,RGB(0,0,0));return (LRESULT)GetStockObject(BLACK_BRUSH);}
 if(msg==WM_PAINT){PAINTSTRUCT ps;HDC dc=BeginPaint(h,&ps);FillRect(dc,&ps.rcPaint,GetStockObject(BLACK_BRUSH));EndPaint(h,&ps);return 0;}
 return DefSubclassProc(h,msg,wp,lp);}
static HWND control(HWND w,const wchar_t *cls,const wchar_t *title,int id,DWORD style){
 HWND h=CreateWindowW(cls,title,WS_CHILD|WS_VISIBLE|style,0,0,10,10,w,(HMENU)(intptr_t)id,GetModuleHandleW(NULL),NULL);SendMessageW(h,WM_SETFONT,(WPARAM)uiFont,TRUE);return h;
}
static void tick(void){
 int s=ap_state(player);if(s==6 && laststate!=6 && ap_index(player)+1<(intptr_t)ap_count(player)){start(ap_index(player)+1);return;}laststate=s;
 int64_t t=ap_time(player),d=ap_length(player);if(t<0)t=0;
 SetWindowTextW(play,s>=1&&s<=3?L"Ⅱ  Pause":L"▶  Play");EnableWindow(position,ap_seekable(player)&&d>0);
 if(!dragging)SendMessageW(position,TBM_SETPOS,TRUE,d>0?t*1000/d:0);
 wchar_t text[100];swprintf(text,100,L"%lld:%02lld / %lld:%02lld",t/60000,t/1000%60,d>0?d/60000:0,d>0?d/1000%60:0);
 SetWindowTextW(clocktext,s==7?L"Playback error":text);
}
static void track(void *menu,int id,const char *name,bool selected){
 HMENU m=menu;wchar_t *title=wide(name);MENUITEMINFOW i={sizeof(i)};i.fMask=MIIM_STRING|MIIM_DATA|MIIM_STATE;i.dwTypeData=title;i.dwItemData=(ULONG_PTR)(intptr_t)id;i.fState=selected?MFS_CHECKED:MFS_UNCHECKED;InsertMenuItemW(m,GetMenuItemCount(m),TRUE,&i);free(title);
}
static HMENU submenu(HMENU parent,const wchar_t *name){
 HMENU m=CreatePopupMenu();AppendMenuW(parent,MF_POPUP,(UINT_PTR)m,name);MENUINFO info={sizeof(info),MIM_STYLE,MNS_NOTIFYBYPOS};SetMenuInfo(m,&info);return m;
}
#include "caption.h"
static LRESULT CALLBACK windowproc(HWND w,UINT msg,WPARAM wp,LPARAM lp){
 switch(msg){
 case WM_NCHITTEST:{
 LRESULT hit=DefWindowProcW(w,msg,wp,lp);if(hit==HTCLIENT&&sevenFrame()){
 POINT pt={(short)LOWORD(lp),(short)HIWORD(lp)};ScreenToClient(w,&pt);
 if(pt.y>=0 && pt.y<scale(32) && ChildWindowFromPointEx(w,pt,CWP_SKIPINVISIBLE)==w)return HTCAPTION;
 }return hit;}
 case WM_NCPAINT:case WM_NCACTIVATE:case WM_SETTEXT:{
 LRESULT result=DefWindowProcW(w,msg,wp,lp);
 if(brandIcon && !sevenFrame())paintCaption(w);if(msg==WM_SETTEXT && sevenFrame())InvalidateRect(w,NULL,TRUE);
 return result;}
 case WM_CREATE:{
 mainwin=w;uiFont=CreateFontW(-scale(14),0,0,0,FW_NORMAL,FALSE,FALSE,FALSE,DEFAULT_CHARSET,0,0,CLEARTYPE_QUALITY,0,L"Segoe UI");updateGlass(w);bar=CreateMenu();HMENU file=CreatePopupMenu();AppendMenuW(bar,MF_POPUP,(UINT_PTR)file,L"File");AppendMenuW(file,MF_STRING,OPEN,L"Open file…");AppendMenuW(file,MF_STRING,STREAM,L"Open network stream…");
 HMENU view=CreatePopupMenu();AppendMenuW(bar,MF_POPUP,(UINT_PTR)view,L"&View");AppendMenuW(view,MF_STRING,FULL,L"Full screen\tF11");
 queue=submenu(bar,L"Queue");audio=submenu(bar,L"Audio track");subs=submenu(bar,L"Subtitles");themeMenu=CreatePopupMenu();AppendMenuW(bar,MF_POPUP,(UINT_PTR)themeMenu,L"&Theme");for(int i=0;i<THEME_COUNT;i++)AppendMenuW(themeMenu,MF_STRING,THEME_FIRST+i,names[i]);SetMenu(w,bar);
 video=control(w,L"STATIC",L"",0,SS_BLACKRECT|WS_CLIPCHILDREN);SetWindowSubclass(video,videoProc,1,0);player=ap_create(video);if(!player)return -1;
 idleLogo=control(video,L"STATIC",L"",0,SS_ICON|SS_CENTERIMAGE);
 brandIcon=LoadImageW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(1),IMAGE_ICON,scale(96),scale(96),0);
 SendMessageW(idleLogo,STM_SETICON,(WPARAM)brandIcon,0);
 control(w,L"BUTTON",L"Open file…",OPEN,BS_OWNERDRAW);control(w,L"BUTTON",L"Theme ▾",THEME_BUTTON,BS_OWNERDRAW);play=control(w,L"BUTTON",L"▶  Play",PLAY,BS_OWNERDRAW);control(w,L"BUTTON",L"Previous",PREV,BS_OWNERDRAW);control(w,L"BUTTON",L"Next",NEXT,BS_OWNERDRAW);volumeLabel=control(w,L"STATIC",L"Volume",0,SS_CENTERIMAGE);
 control(w,L"BUTTON",L"Menu  ▾",CAP_MENU,BS_OWNERDRAW);control(w,L"BUTTON",L"Minimize",CAP_MIN,BS_OWNERDRAW);control(w,L"BUTTON",L"Maximize or restore",CAP_MAX,BS_OWNERDRAW);control(w,L"BUTTON",L"Close",CAP_CLOSE,BS_OWNERDRAW);
 position=control(w,TRACKBAR_CLASSW,L"",SEEK,TBS_NOTICKS);level=control(w,TRACKBAR_CLASSW,L"",VOLUME,TBS_NOTICKS);SetWindowTextW(level,L"Volume");clocktext=control(w,L"STATIC",L"",0,SS_CENTERIMAGE);
 SendMessageW(position,TBM_SETRANGE,TRUE,MAKELONG(0,1000));SendMessageW(level,TBM_SETPOS,TRUE,80);SetTimer(w,1,200,NULL);DragAcceptFiles(w,TRUE);applyTheme(w,savedTheme());return 0;}
 case WM_GETMINMAXINFO:((MINMAXINFO*)lp)->ptMinTrackSize=(POINT){scale(720),scale(340)};return 0;
 case WM_SIZE:if(video)layout(w,LOWORD(lp),HIWORD(lp));return 0;
 case WM_DWMCOMPOSITIONCHANGED:case WM_SETTINGCHANGE:case WM_THEMECHANGED:{
 if(video && themeMenu)applyTheme(w,skin);break;}
 case WM_ERASEBKGND:{
 RECT r;GetClientRect(w,&r);HDC dc=(HDC)wp;Theme t=themes[skin];
 if(contrastMode)FillRect(dc,&r,GetSysColorBrush(COLOR_BTNFACE));
 else if(skin==SEVEN){sevenSurface(dc,w);if(sevenFrame())sevenTitle(w,dc);}

 else{fillColor(dc,r,color(t.panel));if(skin==XP){
 RECT frame=r;frame.bottom-=scale(100);fillColor(dc,frame,color(0x1256ce));
 r.bottom=scale(27);RECT shine=r;shine.bottom=scale(5);blend(dc,shine,0x8bb9ff,0x327df0);
 r.top=shine.bottom;blend(dc,r,0x0867ed,0x1543b2);
 SetBkMode(dc,TRANSPARENT);SetTextColor(dc,RGB(255,255,255));SelectObject(dc,uiFont);r.left=scale(9);DrawTextW(dc,L"RMP",-1,&r,DT_LEFT|DT_SINGLELINE|DT_VCENTER);
 }}
 return 1;}
 case WM_NOTIFY:{
 NMHDR *notice=(NMHDR *)lp;
 if(!contrastMode && skin!=MAC && notice->code==NM_CUSTOMDRAW && (notice->hwndFrom==position||notice->hwndFrom==level)){
 NMCUSTOMDRAW *draw=(NMCUSTOMDRAW *)lp;
 if(draw->dwDrawStage==CDDS_PREPAINT){if(skin==SEVEN)sevenSurface(draw->hdc,notice->hwndFrom);return CDRF_NOTIFYITEMDRAW;}
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
 else if(skin==SEVEN){blend(draw->hdc,r,0xf7fcff,0x71a5d1);DrawEdge(draw->hdc,&r,EDGE_RAISED,BF_RECT);}
 else{fillColor(draw->hdc,r,color(t.accent));}
 return CDRF_SKIPDEFAULT;}
 }}
 break;}
 case WM_DRAWITEM:if(((DRAWITEMSTRUCT*)lp)->CtlType==ODT_BUTTON){if(wp>=CAP_MIN&&wp<=CAP_CLOSE){captionControl((DRAWITEMSTRUCT*)lp);return TRUE;}paintButton((DRAWITEMSTRUCT*)lp);return TRUE;}break;
 case WM_CTLCOLORSTATIC:
 if((HWND)lp==video || (HWND)lp==idleLogo){SetBkColor((HDC)wp,RGB(0,0,0));return (LRESULT)GetStockObject(BLACK_BRUSH);}
 if(skin==SEVEN&&!contrastMode){sevenSurface((HDC)wp,(HWND)lp);SetBkMode((HDC)wp,TRANSPARENT);SetTextColor((HDC)wp,color(0x18324c));return (LRESULT)GetStockObject(NULL_BRUSH);}
 if((HWND)lp!=video && (HWND)lp!=idleLogo && panelBrush && !contrastMode){
 SetBkColor((HDC)wp,color(themes[skin].panel));SetTextColor((HDC)wp,color(themes[skin].text));return (LRESULT)panelBrush;}break;
 case WM_INITMENUPOPUP:{HMENU m=(HMENU)wp;if(m!=queue&&m!=audio&&m!=subs)break;while(GetMenuItemCount(m)>0)DeleteMenu(m,0,MF_BYPOSITION);
 if(m==queue){for(size_t i=0;i<ap_count(player);i++)track(m,(int)i,basename8(ap_item(player,i)),i==(size_t)ap_index(player));}else ap_tracks(player,m==subs,m,track);return 0;}
 case WM_MENUCOMMAND:{HMENU m=(HMENU)lp;MENUITEMINFOW item={sizeof(item)};item.fMask=MIIM_DATA;GetMenuItemInfoW(m,wp,TRUE,&item);if(m==queue)start(item.dwItemData);else ap_select(player,m==subs,(int)item.dwItemData);return 0;}
 case WM_COMMAND:
 if(LOWORD(wp)>=THEME_FIRST && LOWORD(wp)<THEME_FIRST+THEME_COUNT){int value=LOWORD(wp)-THEME_FIRST;applyTheme(w,value);storeTheme(value);return 0;}
 if(LOWORD(wp)==THEME_BUTTON){RECT r;GetWindowRect(GetDlgItem(w,THEME_BUTTON),&r);TrackPopupMenu(themeMenu,TPM_LEFTALIGN|TPM_BOTTOMALIGN,r.left,r.top,0,w,NULL);return 0;}
 switch(LOWORD(wp)){case CAP_MENU:{RECT r;GetWindowRect(GetDlgItem(w,CAP_MENU),&r);
 HMENU popup=CreatePopupMenu();int count=GetMenuItemCount(bar);
 for(int i=0;i<count;i++){wchar_t title[80];GetMenuStringW(bar,i,title,80,MF_BYPOSITION);AppendMenuW(popup,MF_POPUP,(UINT_PTR)GetSubMenu(bar,i),title);}
 TrackPopupMenu(popup,TPM_LEFTALIGN,r.left,r.bottom,0,w,NULL);
 while(GetMenuItemCount(popup)>0)RemoveMenu(popup,0,MF_BYPOSITION);DestroyMenu(popup);break;}
 case CAP_MIN:ShowWindow(w,SW_MINIMIZE);break;case CAP_MAX:ShowWindow(w,IsZoomed(w)?SW_RESTORE:SW_MAXIMIZE);break;case CAP_CLOSE:PostMessageW(w,WM_CLOSE,0,0);break;case OPEN:openfile();break;case PLAY:if(ap_count(player))ap_toggle(player);else openfile();break;case NEXT:start(ap_index(player)+1);break;case PREV:start(ap_index(player)>0?ap_index(player)-1:0);break;case FULL:togglefull();break;case STREAM:DialogBoxW(GetModuleHandleW(NULL),MAKEINTRESOURCEW(200),w,streamdlg);break;}return 0;
 case WM_HSCROLL:if((HWND)lp==level)ap_volume(player,SendMessageW(level,TBM_GETPOS,0,0));else if((HWND)lp==position){dragging=LOWORD(wp)==TB_THUMBTRACK;if(!dragging)ap_seek(player,ap_length(player)*SendMessageW(position,TBM_GETPOS,0,0)/1000);}return 0;
 case WM_TIMER:
 tick();
 if(smoke && ((ap_state(player)==3 && ap_time(player)>100) || GetTickCount64()>deadline)){
 exitcode=ap_state(player)==3 && ap_time(player)>100?0:2;DestroyWindow(w);
 }return 0;
 case WM_DROPFILES:{HDROP drop=(HDROP)wp;size_t first=ap_count(player);UINT n=DragQueryFileW(drop,0xffffffff,NULL,0);for(UINT i=0;i<n;i++){wchar_t path[32768];DragQueryFileW(drop,i,path,32768);enqueue(path,false);}DragFinish(drop);start(first);return 0;}
 case WM_DESTROY:KillTimer(w,1);if(player)ap_destroy(player);DeleteObject(panelBrush);DestroyIcon(brandIcon);DeleteObject(uiFont);PostQuitMessage(exitcode);return 0;
 }
 return DefWindowProcW(w,msg,wp,lp);
}
#include "update.h"
int WINAPI wWinMain(HINSTANCE h,HINSTANCE prev,LPWSTR cmd,int show){
 SetProcessDPIAware();INITCOMMONCONTROLSEX controls={sizeof(controls),ICC_BAR_CLASSES};InitCommonControlsEx(&controls);
 WNDCLASSW c={0};c.lpfnWndProc=windowproc;c.hInstance=h;c.lpszClassName=L"RMPPlayer";c.hIcon=LoadIconW(h,MAKEINTRESOURCEW(1));c.hCursor=LoadCursor(NULL,IDC_ARROW);c.hbrBackground=(HBRUSH)(COLOR_BTNFACE+1);RegisterClassW(&c);
 HWND w=CreateWindowW(c.lpszClassName,L"RMP · Rust Media Player",WS_OVERLAPPEDWINDOW|WS_CLIPCHILDREN,CW_USEDEFAULT,CW_USEDEFAULT,900,570,NULL,NULL,h,NULL);if(!w)return 1;
 int argc;wchar_t **argv=CommandLineToArgvW(GetCommandLineW(),&argc);smoke=argc==3 && wcscmp(argv[1],L"--smoke-test")==0;deadline=GetTickCount64()+20000;for(int i=smoke?2:1;i<argc;i++)enqueue(argv[i],wcsstr(argv[i],L"://")!=NULL);LocalFree(argv);if(argc>1)start(0);
 ShowWindow(w,show);if(!smoke)checkUpdates();MSG m;while(GetMessageW(&m,NULL,0,0)>0){if(m.message==WM_KEYDOWN && (m.wParam==VK_F11 || (m.wParam==VK_ESCAPE && fullscreen))){togglefull();continue;}TranslateMessage(&m);DispatchMessageW(&m);}return (int)m.wParam;
}
