static HWND transport;
static POINT lastMouse;
static ULONGLONG hideAt;
static LRESULT CALLBACK transportProc(HWND w,UINT msg,WPARAM wp,LPARAM lp){
 switch(msg){
 case WM_COMMAND:case WM_HSCROLL:case WM_NOTIFY:case WM_DRAWITEM:case WM_CTLCOLORSTATIC:
 return SendMessageW(mainwin,msg,wp,lp);
 case WM_ERASEBKGND:{RECT r;GetClientRect(w,&r);FillRect((HDC)wp,&r,panelBrush?panelBrush:GetSysColorBrush(COLOR_BTNFACE));return 1;}
 case WM_MOUSEACTIVATE:return MA_NOACTIVATE;
 }
 return DefWindowProcW(w,msg,wp,lp);
}
static void moveTransport(HWND from,HWND to){
 int ids[]={OPEN,THEME_BUTTON,PREV,PLAY,NEXT,FULL,STOP,LIST,REPEAT,SHUFFLE};
 for(unsigned i=0;i<sizeof(ids)/sizeof(ids[0]);i++){HWND b=GetDlgItem(from,ids[i]);if(b)SetParent(b,to);}
 HWND rest[]={position,level,volumeLabel,clocktext};for(unsigned i=0;i<4;i++)SetParent(rest[i],to);
}
static void updateTransport(void){
 if(!fullscreen||!transport)return;
 POINT mouse;GetCursorPos(&mouse);RECT r;GetWindowRect(transport,&r);
 bool moved=mouse.x!=lastMouse.x||mouse.y!=lastMouse.y;lastMouse=mouse;
 if(moved||(IsWindowVisible(transport)&&PtInRect(&r,mouse))||GetCapture()){hideAt=GetTickCount64()+2500;ShowWindow(transport,SW_SHOWNOACTIVATE);}
 else if(GetTickCount64()>=hideAt)ShowWindow(transport,SW_HIDE);
}
