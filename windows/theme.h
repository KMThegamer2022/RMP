#include "../themes.h"
static int skin=VLC_THEME;
static HMENU themeMenu;
static HBRUSH panelBrush;
static bool contrastMode;
enum { THEME_BUTTON=300, THEME_FIRST=310 };
static const wchar_t *names[]={L"Windows 10/11",L"Windows 7",L"Windows XP",L"Mac",L"RMP Default",L"VLC"};
static COLORREF color(unsigned c){return RGB(c>>16,(c>>8)&255,c&255);}
static int savedTheme(void){
 DWORD value=0,size=sizeof(value);
 if(RegGetValueW(HKEY_CURRENT_USER,L"Software\\RMP",L"Theme",RRF_RT_REG_DWORD,NULL,&value,&size)!=ERROR_SUCCESS || value>=THEME_COUNT)return VLC_THEME;
 return (int)value;
}
static void storeTheme(int value){
 HKEY key;if(RegCreateKeyExW(HKEY_CURRENT_USER,L"Software\\RMP",0,NULL,0,KEY_SET_VALUE,NULL,&key,NULL)==ERROR_SUCCESS){
 DWORD d=value;RegSetValueExW(key,L"Theme",0,REG_DWORD,(BYTE*)&d,sizeof(d));RegCloseKey(key);}
}
static void fillColor(HDC dc,RECT r,COLORREF c){SetDCBrushColor(dc,c);FillRect(dc,&r,GetStockObject(DC_BRUSH));}
static void blend(HDC dc,RECT r,unsigned a,unsigned b){
 TRIVERTEX v[2]={{r.left,r.top,((a>>16)&255)<<8,((a>>8)&255)<<8,(a&255)<<8,0},
 {r.right,r.bottom,((b>>16)&255)<<8,((b>>8)&255)<<8,(b&255)<<8,0}};
 GRADIENT_RECT g={0,1};GradientFill(dc,v,2,&g,1,GRADIENT_FILL_RECT_V);
}
static void paintButton(DRAWITEMSTRUCT *d){
 Theme t=themes[skin];if(skin==RMP_DEFAULT && d->CtlID==PLAY){t.face=t.accent;t.edge=0xdd6238;t.text=0x16191f;}RECT r=d->rcItem;HDC dc=d->hDC;
 if(contrastMode){DrawFrameControl(dc,&r,DFC_BUTTON,DFCS_BUTTONPUSH|((d->itemState&ODS_SELECTED)?DFCS_PUSHED:0));}
 else{
 fillColor(dc,r,color(t.panel));InflateRect(&r,-1,-1);
 int corner=scale(t.radius*2);
 HRGN region=CreateRoundRectRgn(r.left,r.top,r.right+1,r.bottom+1,corner,corner);
 int saved=SaveDC(dc);ExtSelectClipRgn(dc,region,RGN_AND);
 if(skin==XP)blend(dc,r,t.highlight,(d->itemState&ODS_SELECTED)?t.edge:t.face);
 else fillColor(dc,r,color((d->itemState&ODS_SELECTED)?t.edge:t.face));
 RestoreDC(dc,saved);SetDCBrushColor(dc,color(t.edge));if(skin!=MODERN)FrameRgn(dc,region,GetStockObject(DC_BRUSH),1,1);DeleteObject(region);
 }
 wchar_t text[80];GetWindowTextW(d->hwndItem,text,80);SelectObject(dc,uiFont);
 SetBkMode(dc,TRANSPARENT);SetTextColor(dc,contrastMode?GetSysColor(COLOR_BTNTEXT):color(t.text));
 if(!contrastMode && skin!=MAC){
 if(d->CtlID==PLAY){wcscpy(text,wcsstr(text,L"Pause")?L"Ⅱ":L"▶");}
 if(d->CtlID==PREV)wcscpy(text,L"❘◀");
 if(d->CtlID==NEXT)wcscpy(text,L"▶❘");
 }
 DrawTextW(dc,text,-1,&r,DT_CENTER|DT_VCENTER|DT_SINGLELINE);
 if(d->itemState&ODS_FOCUS){InflateRect(&r,-3,-3);DrawFocusRect(dc,&r);}
}
