/* Paint native theme parts into a 32-bit surface. Untouched pixels expose DWM. */
static bool paintGlassControl(HWND h,HDC dc){
 HTHEME theme=OpenThemeData(h,L"Trackbar");
 if(!theme)return false;
 RECT r;GetClientRect(h,&r);
 BP_PAINTPARAMS params={sizeof(params),BPPF_ERASE};
 HDC surface;
 HPAINTBUFFER buffer=BeginBufferedPaint(dc,&r,BPBF_TOPDOWNDIB,&params,&surface);
 if(!buffer){CloseThemeData(theme);return false;}
 HRESULT result=S_OK;
 bool enabled=IsWindowEnabled(h);
 POINT cursor;GetCursorPos(&cursor);ScreenToClient(h,&cursor);
 bool hot=PtInRect(&r,cursor);
  RECT channel,thumb;
  SendMessageW(h,TBM_GETCHANNELRECT,0,(LPARAM)&channel);
  SendMessageW(h,TBM_GETTHUMBRECT,0,(LPARAM)&thumb);
  result=DrawThemeBackground(theme,surface,TKP_TRACK,TRS_NORMAL,&channel,NULL);
  int state=!enabled?TUS_DISABLED:GetCapture()==h?TUS_PRESSED:PtInRect(&thumb,cursor)?TUS_HOT:GetFocus()==h?TUS_FOCUSED:TUS_NORMAL;
  if(SUCCEEDED(result))result=DrawThemeBackground(theme,surface,TKP_THUMBBOTTOM,state,&thumb,NULL);
 if(GetFocus()==h && !(SendMessageW(h,WM_QUERYUISTATE,0,0)&UISF_HIDEFOCUS)){
  RECT focus=r;InflateRect(&focus,-2,-2);
  // Give the keyboard focus outline a visible alpha channel on glass.
  FrameRect(surface,&focus,GetSysColorBrush(COLOR_HIGHLIGHT));
  RECT edges[]={ {focus.left,focus.top,focus.right,focus.top+1},{focus.left,focus.bottom-1,focus.right,focus.bottom},
                 {focus.left,focus.top,focus.left+1,focus.bottom},{focus.right-1,focus.top,focus.right,focus.bottom} };
  for(int i=0;i<4;i++)BufferedPaintSetAlpha(buffer,&edges[i],255);
 }
 EndBufferedPaint(buffer,SUCCEEDED(result));CloseThemeData(theme);
 return SUCCEEDED(result);
}
static LRESULT CALLBACK glassControlProc(HWND h,UINT msg,WPARAM wp,LPARAM lp,UINT_PTR id,DWORD_PTR data){
 if(h==level && msg==WM_MOUSEWHEEL){
  static int wheelRemainder;
  wheelRemainder+=GET_WHEEL_DELTA_WPARAM(wp);
  int steps=wheelRemainder/WHEEL_DELTA;wheelRemainder%=WHEEL_DELTA;
  int volume=volumePercent+steps*5;
  volumePercent=max(0,min(100,volume));
  SendMessageW(h,TBM_SETPOS,TRUE,volumePercent);ap_volume(player,volumePercent);
  return 0;
 }
 if(glass && GetParent(h)==mainwin && (h==position||h==level)){
  if(msg==WM_ERASEBKGND)return 1;
  if(msg==WM_PAINT){
   PAINTSTRUCT ps;HDC dc=BeginPaint(h,&ps);
   if(!paintGlassControl(h,dc))DefSubclassProc(h,WM_PRINTCLIENT,(WPARAM)dc,PRF_CLIENT|PRF_ERASEBKGND);
   EndPaint(h,&ps);return 0;
  }
  if(msg==WM_PRINTCLIENT && paintGlassControl(h,(HDC)wp))return 0;
 }
 return DefSubclassProc(h,msg,wp,lp);
}
