/* Keep the standard system hit targets, sizing frame and keyboard commands.
   Draw using the button rectangles reported by the accessibility provider. */
static void paintCaption(HWND w){
 if(fullscreen||contrastMode||skin!=XP)return;
 TITLEBARINFOEX info={sizeof(info)};
 SendMessageW(w,WM_GETTITLEBARINFOEX,0,(LPARAM)&info);
 RECT outer;GetWindowRect(w,&outer);
 RECT close=info.rgrect[5];
 if(IsRectEmpty(&close))return;
 OffsetRect(&close,-outer.left,-outer.top);
 RECT band={scale(4),max(0,close.top),outer.right-outer.left-scale(4),close.bottom+scale(2)};
 HDC dc=GetWindowDC(w);if(!dc)return;
 int saved=SaveDC(dc);bool active=GetForegroundWindow()==w;
 blend(dc,band,active?0x428afa:0x86a8d8,active?0x0751ca:0x7092ca);
 RECT title=band;title.left+=scale(25);title.right=info.rgrect[2].left-outer.left-scale(4);
 DrawIconEx(dc,band.left+scale(4),band.top+scale(3),brandIcon,scale(16),scale(16),0,NULL,DI_NORMAL);
 wchar_t name[512];GetWindowTextW(w,name,512);SelectObject(dc,uiFont);SetBkMode(dc,TRANSPARENT);
 SetTextColor(dc,RGB(255,255,255));DrawTextW(dc,name,-1,&title,DT_SINGLELINE|DT_VCENTER|DT_END_ELLIPSIS|DT_NOPREFIX);
 for(int i=2;i<=5;i++){if(i==4)continue;RECT r=info.rgrect[i];if(IsRectEmpty(&r))continue;
 OffsetRect(&r,-outer.left,-outer.top);InflateRect(&r,-1,-1);
 bool red=i==5;blend(dc,r,red?0xffaa89:0xafcef5,red?0xb32d18:0x225bd1);
 SetDCBrushColor(dc,RGB(240,246,255));FrameRect(dc,&r,GetStockObject(DC_BRUSH));
 SetTextColor(dc,RGB(255,255,255));DrawTextW(dc,i==2?L"−":i==3?(IsZoomed(w)?L"❐":L"□"):L"×",-1,&r,DT_SINGLELINE|DT_CENTER|DT_VCENTER);}
 RestoreDC(dc,saved);ReleaseDC(w,dc);
}
