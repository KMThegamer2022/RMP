/* A native terminal for RMP's command language. All input goes directly to
   the shared Rust command parser; the window does not start a system shell. */
enum { SHOW_TERMINAL=620, TERM_OUTPUT, TERM_PROMPT, TERM_INPUT, REVEAL_TERMINAL=WM_APP+70 };
extern char *ap_terminal_command(Player *,const char *,int *);
extern char *ap_terminal_complete(Player *,const char *);
extern char *ap_terminal_prompt(Player *);
extern int ap_terminal_volume(Player *);
extern void ap_terminal_free(char *);

static HWND terminalWindow,terminalOutput,terminalPrompt,terminalInput;
static HFONT terminalFont;
static HBRUSH terminalBrush;
static WCHAR *terminalHistory[64],*terminalDraft;
static int historyCount,historyIndex;
static const COLORREF terminalBackground=RGB(12,20,16);

static char *terminalUtf8(const WCHAR *line){
 int size=WideCharToMultiByte(CP_UTF8,0,line,-1,NULL,0,NULL,NULL);
 char *utf8=size?malloc(size):NULL;
 if(utf8)WideCharToMultiByte(CP_UTF8,0,line,-1,utf8,size,NULL,NULL);
 return utf8;
}
static WCHAR *terminalLine(HWND h){
 int len=GetWindowTextLengthW(h);
 WCHAR *line=calloc((size_t)len+1,sizeof(*line));
 if(line)GetWindowTextW(h,line,len+1);
 return line;
}
static void appendTerminalText(const WCHAR *text){
 if(!text||!*text)return;
 size_t len=wcslen(text);
 WCHAR *normalized=malloc((len*2+1)*sizeof(*normalized));
 if(!normalized)return;
 size_t end=0;
 for(size_t i=0;i<len;i++){
  if(text[i]==L'\n' && (!i||text[i-1]!=L'\r'))normalized[end++]=L'\r';
  normalized[end++]=text[i];
 }
 normalized[end]=0;
 enum { TRANSCRIPT_LIMIT=120000 };
 int existing=GetWindowTextLengthW(terminalOutput);
 if(existing+(int)end>TRANSCRIPT_LIMIT){
  int remove=existing+(int)end-TRANSCRIPT_LIMIT;
  LRESULT line=SendMessageW(terminalOutput,EM_LINEFROMCHAR,min(remove,existing),0);
  LRESULT next=SendMessageW(terminalOutput,EM_LINEINDEX,line+1,0);
  SendMessageW(terminalOutput,EM_SETSEL,0,next<0?existing:next);
  SendMessageW(terminalOutput,EM_REPLACESEL,FALSE,(LPARAM)L"");
 }
 SendMessageW(terminalOutput,EM_SETSEL,-1,-1);
 const WCHAR *tail=end>TRANSCRIPT_LIMIT?normalized+end-TRANSCRIPT_LIMIT:normalized;
 SendMessageW(terminalOutput,EM_REPLACESEL,FALSE,(LPARAM)tail);
 SendMessageW(terminalOutput,EM_SCROLLCARET,0,0);free(normalized);
}
static void layoutTerminal(HWND w){
 if(!terminalInput)return;
 RECT r;GetClientRect(w,&r);
 int margin=scale(12),line=scale(24),available=max(1,r.right-2*margin);
 WCHAR *prompt=terminalLine(terminalPrompt);
 HDC dc=GetDC(terminalPrompt);HGDIOBJ old=SelectObject(dc,terminalFont);SIZE extent={0};
 if(prompt)GetTextExtentPoint32W(dc,prompt,wcslen(prompt),&extent);
 SelectObject(dc,old);ReleaseDC(terminalPrompt,dc);free(prompt);
 int prefix=min(available*3/5,extent.cx+scale(3)),gap=scale(4);
 MoveWindow(terminalOutput,margin,margin,available,max(1,r.bottom-3*margin-line),TRUE);
 MoveWindow(terminalPrompt,margin,r.bottom-margin-line,prefix,line,TRUE);
 MoveWindow(terminalInput,margin+prefix+gap,r.bottom-margin-line,max(1,available-prefix-gap),line,TRUE);
}
static void refreshTerminalPrompt(void){
 char *prompt=ap_terminal_prompt(player);
 if(prompt){WCHAR *value=wide(prompt);if(value)SetWindowTextW(terminalPrompt,value);free(value);ap_terminal_free(prompt);layoutTerminal(GetParent(terminalPrompt));}
}
static void setTerminalInput(const WCHAR *line){
 SetWindowTextW(terminalInput,line);SendMessageW(terminalInput,EM_SETSEL,-1,-1);
}
static void rememberTerminalLine(const WCHAR *line){
 if(!historyCount||wcscmp(terminalHistory[historyCount-1],line)){
  WCHAR *copy=_wcsdup(line);
  if(copy){
   if(historyCount==64){free(terminalHistory[0]);memmove(terminalHistory,terminalHistory+1,63*sizeof(*terminalHistory));historyCount--;}
   terminalHistory[historyCount++]=copy;
  }
 }
 historyIndex=historyCount;free(terminalDraft);terminalDraft=NULL;
}
static void navigateHistory(int delta){
 if(!historyCount)return;
 int next=max(0,min(historyCount,historyIndex+delta));
 if(next==historyIndex)return;
 if(historyIndex==historyCount){free(terminalDraft);terminalDraft=terminalLine(terminalInput);}
 historyIndex=next;
 setTerminalInput(next<historyCount?terminalHistory[next]:terminalDraft?terminalDraft:L"");
}
static void runTerminalInput(bool complete){
 WCHAR *line=terminalLine(terminalInput);if(!line)return;
 if(!complete&&!*line){free(line);return;}
 char *utf8=terminalUtf8(line);if(!utf8){free(line);return;}
 int action=0;
 char *response=complete?ap_terminal_complete(player,utf8):ap_terminal_command(player,utf8,&action);
 WCHAR *result=response?wide(response):NULL;
 free(utf8);if(response)ap_terminal_free(response);
 if(complete){if(result)setTerminalInput(result);}
 else{
  WCHAR *prefix=terminalLine(terminalPrompt);
  appendTerminalText(prefix);appendTerminalText(L" ");appendTerminalText(line);appendTerminalText(L"\r\n");free(prefix);
  rememberTerminalLine(line);setTerminalInput(L"");
  if(action==2)SetWindowTextW(terminalOutput,L"");
  if(result&&*result){appendTerminalText(result);if(result[wcslen(result)-1]!=L'\n')appendTerminalText(L"\r\n");}
  if(!result)appendTerminalText(L"RMP could not process the command.\r\n");
  refreshTerminalPrompt();
  if(action==9 || (action>=10 && action<10+THEME_COUNT)){
   int choice=action==9?previousTheme():action-10;
   SendMessageW(mainwin,WM_COMMAND,THEME_FIRST+choice,0);
   if(choice!=LINUX_THEME)SetForegroundWindow(mainwin);
  }
  switch(action){
   case 1:PostMessageW(mainwin,WM_COMMAND,FULL,0);break;
   case 3:showCurrentTrack();break;
   case 4:volumePercent=ap_terminal_volume(player);SendMessageW(level,TBM_SETPOS,TRUE,volumePercent);break;
  }
 }
 free(result);free(line);
}
static LRESULT CALLBACK terminalEditProc(HWND h,UINT msg,WPARAM wp,LPARAM lp,UINT_PTR id,DWORD_PTR data){
 if(msg==WM_KEYDOWN){
  switch(wp){
   case VK_RETURN:runTerminalInput(false);return 0;
   case VK_TAB:runTerminalInput(true);return 0;
   case VK_UP:navigateHistory(-1);return 0;
   case VK_DOWN:navigateHistory(1);return 0;
  }
 }
 if(msg==WM_CHAR&&(wp==L'\r'||wp==L'\t'))return 0;
 return DefSubclassProc(h,msg,wp,lp);
}
static HWND terminalChild(HWND w,const WCHAR *cls,const WCHAR *label,int id,DWORD style){
 HWND h=CreateWindowW(cls,label,WS_CHILD|WS_VISIBLE|style,0,0,1,1,w,(HMENU)(INT_PTR)id,GetModuleHandleW(NULL),NULL);
 SendMessageW(h,WM_SETFONT,(WPARAM)terminalFont,FALSE);return h;
}
static LRESULT CALLBACK terminalProc(HWND w,UINT msg,WPARAM wp,LPARAM lp){
 switch(msg){
 case WM_CREATE:{
  terminalBrush=CreateSolidBrush(terminalBackground);
  terminalFont=CreateFontW(-scale(15),0,0,0,FW_NORMAL,FALSE,FALSE,FALSE,DEFAULT_CHARSET,0,0,CLEARTYPE_QUALITY,FIXED_PITCH|FF_MODERN,L"Consolas");
  terminalOutput=terminalChild(w,L"EDIT",L"",TERM_OUTPUT,ES_MULTILINE|ES_AUTOVSCROLL|ES_READONLY|WS_VSCROLL);
  terminalPrompt=terminalChild(w,L"STATIC",L"",TERM_PROMPT,SS_LEFTNOWORDWRAP|SS_NOPREFIX|SS_PATHELLIPSIS);
  terminalInput=terminalChild(w,L"EDIT",L"",TERM_INPUT,ES_AUTOHSCROLL|WS_TABSTOP);
  if(!terminalOutput||!terminalPrompt||!terminalInput)return -1;
  SendMessageW(terminalOutput,EM_SETLIMITTEXT,150000,0);
  SendMessageW(terminalInput,EM_SETLIMITTEXT,32767,0);
  SetWindowSubclass(terminalInput,terminalEditProc,1,0);
  appendTerminalText(L"RMP Terminal\r\nType help for commands.\r\n\r\n");refreshTerminalPrompt();return 0;
 }
 case WM_SIZE:layoutTerminal(w);return 0;
 case WM_GETMINMAXINFO:((MINMAXINFO*)lp)->ptMinTrackSize=(POINT){scale(300),scale(170)};return 0;
 case WM_SETFOCUS:SetFocus(terminalInput);return 0;
 case WM_CTLCOLOREDIT:case WM_CTLCOLORSTATIC:
  SetBkColor((HDC)wp,terminalBackground);SetTextColor((HDC)wp,(HWND)lp==terminalPrompt?RGB(117,230,151):RGB(219,236,224));
  return (LRESULT)terminalBrush;
 case WM_ERASEBKGND:{RECT r;GetClientRect(w,&r);FillRect((HDC)wp,&r,terminalBrush);return 1;}
 case WM_CLOSE:ShowWindow(w,SW_HIDE);SetForegroundWindow(mainwin);return 0;
 case WM_DESTROY:
  for(int i=0;i<historyCount;i++)free(terminalHistory[i]);
  historyCount=0;historyIndex=0;free(terminalDraft);terminalDraft=NULL;
  DeleteObject(terminalFont);DeleteObject(terminalBrush);terminalWindow=NULL;return 0;
 }
 return DefWindowProcW(w,msg,wp,lp);
}
static void revealTerminal(void){
 if(skin!=LINUX_THEME)return;
 if(!terminalWindow){
  MONITORINFO monitor={sizeof(monitor)};GetMonitorInfoW(MonitorFromWindow(mainwin,MONITOR_DEFAULTTONEAREST),&monitor);
  RECT playerRect;GetWindowRect(mainwin,&playerRect);
  RECT work=monitor.rcWork;int gap=scale(9);
  int width=min(scale(640),work.right-work.left),height=min(scale(320),work.bottom-work.top);
  int x=work.right-width,y=work.bottom-height;
  if(work.right-playerRect.right>=scale(390)){
   x=playerRect.right+gap;width=min(width,work.right-x);y=max(work.top,min(playerRect.top,y));
  }else if(work.bottom-playerRect.bottom>=scale(190)){
   y=playerRect.bottom+gap;height=min(height,work.bottom-y);x=max(work.left,min(playerRect.left,x));
  }
  terminalWindow=CreateWindowExW(WS_EX_APPWINDOW,L"RMPTerminal",L"RMP Terminal",WS_OVERLAPPEDWINDOW|WS_CLIPCHILDREN,
   x,y,width,height,NULL,NULL,GetModuleHandleW(NULL),NULL);
  if(!terminalWindow){MessageBoxW(mainwin,L"Could not open the RMP terminal.",L"RMP",MB_ICONERROR);return;}
 }
 ShowWindow(terminalWindow,IsIconic(terminalWindow)?SW_RESTORE:SW_SHOW);
 SetForegroundWindow(terminalWindow);SetFocus(terminalInput);
}
