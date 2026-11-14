enum { CAP_MIN=410,CAP_MAX,CAP_CLOSE,CAP_MENU };
static bool sevenFrame(void){return skin==SEVEN&&!contrastMode&&!fullscreen;}
/* Premultiplied BGRA carries the tint to DWM. Even over a black or
   unavailable backdrop the tint remains visible. Tiles share window coordinates. */
static HDC sheenDC;
static HBITMAP sheenBitmap,oldSheenBitmap;
static bool loadSheen(void){
 if(sheenDC)return true;
 BITMAPINFO info={0};info.bmiHeader.biSize=sizeof(BITMAPINFOHEADER);
 info.bmiHeader.biWidth=384;info.bmiHeader.biHeight=-128;
 info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=32;
 DWORD *pixels=NULL;
 sheenBitmap=CreateDIBSection(NULL,&info,DIB_RGB_COLORS,(void**)&pixels,NULL,0);
 if(!sheenBitmap)return false;
 sheenDC=CreateCompatibleDC(NULL);
 if(!sheenDC){DeleteObject(sheenBitmap);sheenBitmap=NULL;return false;}
 oldSheenBitmap=SelectObject(sheenDC,sheenBitmap);
 for(int y=0;y<128;y++)for(int x=0;x<384;x++){
 int diagonal=(x+y*2)%384;
 int reflection=max(0,48-abs(diagonal-100))+max(0,18-abs(diagonal-184));
 int gloss=y<59?26-y/3:5+(y-59)/6;
 int r=min(255,146+gloss+reflection),g=min(255,183+gloss+reflection),b=min(255,207+gloss+reflection);
 int alpha=193+reflection/2;
 pixels[y*384+x]=(DWORD)alpha<<24|(DWORD)(r*alpha/255)<<16|(DWORD)(g*alpha/255)<<8|(b*alpha/255);
 }
 return true;
}
static void freeSheen(void){if(sheenDC){SelectObject(sheenDC,oldSheenBitmap);DeleteObject(sheenBitmap);DeleteDC(sheenDC);sheenDC=NULL;}}
static void drawGlassBand(HDC dc,RECT band){
 if(IsRectEmpty(&band))return;
 if(!glass||!loadSheen()){blend(dc,band,0xe8f6ff,0xb4cfe4);return;}
 BLENDFUNCTION mode={AC_SRC_OVER,0,255,AC_SRC_ALPHA};
 fillColor(dc,band,RGB(0,0,0));int saved=SaveDC(dc);IntersectClipRect(dc,band.left,band.top,band.right,band.bottom);
 for(int x=band.left;x<band.right;x+=384)AlphaBlend(dc,x,band.top,384,band.bottom-band.top,sheenDC,0,0,384,128,mode);
 RestoreDC(dc,saved);
}
static void sevenSurface(HDC dc,HWND child){
 RECT area;GetClientRect(mainwin,&area);
 POINT offset={0};if(child!=mainwin)MapWindowPoints(child,mainwin,&offset,1);
 int save=SaveDC(dc);SetViewportOrgEx(dc,-offset.x,-offset.y,NULL);
 fillColor(dc,area,color(0xbdd4e7));
 RECT top=area;top.bottom=scale(32);drawGlassBand(dc,top);
 RECT command=area;command.top=scale(32);command.bottom=scale(56);
 blend(dc,command,0xf9fcff,0xdce7f2);
 RECT deck=area;deck.top=deck.bottom-scale(100);
 drawGlassBand(dc,deck);
 RECT line=deck;line.bottom=line.top+1;fillColor(dc,line,RGB(255,255,255));
 RECT edge=area;InflateRect(&edge,-1,-1);SetDCBrushColor(dc,color(0xf3fbff));FrameRect(dc,&edge,GetStockObject(DC_BRUSH));
 RestoreDC(dc,save);
}
static void captionControl(DRAWITEMSTRUCT *d){
 RECT r=d->rcItem;bool close=d->CtlID==CAP_CLOSE;int mid=(r.bottom+r.top)/2;
 POINT cursor;GetCursorPos(&cursor);bool hot=WindowFromPoint(cursor)==d->hwndItem;
 RECT upper=r,lower=r;upper.bottom=lower.top=mid;
 blend(d->hDC,upper,hot?0xffffff:close?0xffd3c2:0xf4fcff,hot?(close?0xff7952:0x73cafa):close?0xf68b73:0xc4dae9);
 blend(d->hDC,lower,close?0xdb543a:0x9cbad0,close?0xb83b26:0xbed7ea);
 SetDCBrushColor(d->hDC,color(close?0x914630:0x768da2));FrameRect(d->hDC,&r,GetStockObject(DC_BRUSH));
 SetBkMode(d->hDC,TRANSPARENT);SetTextColor(d->hDC,color(close?0xffffff:0x233e50));SelectObject(d->hDC,uiFont);
 DrawTextW(d->hDC,close?L"×":d->CtlID==CAP_MIN?L"−":IsZoomed(mainwin)?L"❐":L"□",-1,&r,DT_CENTER|DT_SINGLELINE|DT_VCENTER);
}
static void sevenTitle(HWND w,HDC dc){
 RECT r;GetClientRect(w,&r);r.left=scale(29);r.right-=scale(125);r.bottom=scale(30);
 wchar_t title[512];GetWindowTextW(w,title,512);SelectObject(dc,uiFont);SetBkMode(dc,TRANSPARENT);SetTextColor(dc,color(0x152d42));
 DrawIconEx(dc,scale(8),scale(7),brandIcon,scale(16),scale(16),0,NULL,DI_NORMAL);
 UINT format=DT_SINGLELINE|DT_VCENTER|DT_END_ELLIPSIS|DT_NOPREFIX;
 SetTextColor(dc,color(0xf1faff));for(int dy=-1;dy<=1;dy++)for(int dx=-1;dx<=1;dx++){RECT halo=r;OffsetRect(&halo,dx,dy);DrawTextW(dc,title,-1,&halo,format);}
 SetTextColor(dc,color(0x183149));DrawTextW(dc,title,-1,&r,format);
}
