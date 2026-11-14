/* VLC owns its video child on another thread. Foreground raw input lets the
   player recognize clicks there without intercepting other windows' messages. */
static void videoMouseInput(HRAWINPUT input){
 RAWINPUT event;UINT bytes=sizeof(event);
 if(GetRawInputData(input,RID_INPUT,&event,&bytes,sizeof(RAWINPUTHEADER))==(UINT)-1 ||
    event.header.dwType!=RIM_TYPEMOUSE || !(event.data.mouse.usButtonFlags&RI_MOUSE_LEFT_BUTTON_DOWN))return;
 static bool pending;
 static DWORD firstTime;
 static POINT firstPoint;
 POINT point;GetCursorPos(&point);
 HWND hit=WindowFromPoint(point);
 if(hit!=video && !IsChild(video,hit)){pending=false;return;}
 DWORD time=(DWORD)GetMessageTime();
 if(pending && time-firstTime<=GetDoubleClickTime() &&
    abs(point.x-firstPoint.x)<=GetSystemMetrics(SM_CXDOUBLECLK)/2 &&
    abs(point.y-firstPoint.y)<=GetSystemMetrics(SM_CYDOUBLECLK)/2){
  pending=false;PostMessageW(mainwin,WM_COMMAND,FULL,0);
 }else{pending=true;firstTime=time;firstPoint=point;}
}
