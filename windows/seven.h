enum { CAP_MIN=410,CAP_MAX,CAP_CLOSE };
static bool sevenFrame(void){return skin==SEVEN&&!contrastMode&&!fullscreen;}
static void sevenSurface(HDC dc,HWND child){
 RECT area;GetClientRect(mainwin,&area);
 POINT offset={0};if(child!=mainwin)MapWindowPoints(child,mainwin,&offset,1);
 int save=SaveDC(dc);SetViewportOrgEx(dc,-offset.x,-offset.y,NULL);
 fillColor(dc,area,color(0xbdd4e7));
 RECT top=area;top.bottom=scale(32);blend(dc,top,0xeaf6ff,0x9bbbd4);
 RECT deck=area;deck.top=deck.bottom-scale(100);
 RECT upper=deck;upper.bottom=deck.top+scale(34);blend(dc,upper,0xf7fcff,0xe5eff8);
 deck.top=upper.bottom;blend(dc,deck,0xddebf7,0xc8daeb);
 RestoreDC(dc,save);
}
static void captionControl(DRAWITEMSTRUCT *d){
 RECT r=d->rcItem;bool close=d->CtlID==CAP_CLOSE;int mid=(r.bottom+r.top)/2;
 RECT upper=r,lower=r;upper.bottom=lower.top=mid;
 blend(d->hDC,upper,close?0xffd3c2:0xf4fcff,close?0xf68b73:0xc4dae9);
 blend(d->hDC,lower,close?0xdb543a:0x9cbad0,close?0xb83b26:0xbed7ea);
 SetDCBrushColor(d->hDC,color(close?0x914630:0x768da2));FrameRect(d->hDC,&r,GetStockObject(DC_BRUSH));
 SetBkMode(d->hDC,TRANSPARENT);SetTextColor(d->hDC,color(close?0xffffff:0x233e50));SelectObject(d->hDC,uiFont);
 DrawTextW(d->hDC,close?L"×":d->CtlID==CAP_MIN?L"−":IsZoomed(mainwin)?L"❐":L"□",-1,&r,DT_CENTER|DT_SINGLELINE|DT_VCENTER);
}
static void sevenTitle(HWND w,HDC dc){
 RECT r;GetClientRect(w,&r);r.left=scale(29);r.right-=scale(125);r.bottom=scale(30);
 wchar_t title[512];GetWindowTextW(w,title,512);SelectObject(dc,uiFont);SetBkMode(dc,TRANSPARENT);SetTextColor(dc,color(0x152d42));
 DrawIconEx(dc,scale(8),scale(7),brandIcon,scale(16),scale(16),0,NULL,DI_NORMAL);
 DrawTextW(dc,title,-1,&r,DT_SINGLELINE|DT_VCENTER|DT_END_ELLIPSIS|DT_NOPREFIX);
}
