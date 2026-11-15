// Rust owns command parsing, completion and the working directory.
static NSString *takeTerminalString(char *text){
 if(!text)return @"";
 NSString *value=[NSString stringWithUTF8String:text]?:@"";
 ap_terminal_free(text);
 return value;
}
-(void)fitTerminalText{
 [terminalTranscript.layoutManager ensureLayoutForTextContainer:terminalTranscript.textContainer];
 terminalTextHeight.constant=ceil([terminalTranscript.layoutManager usedRectForTextContainer:terminalTranscript.textContainer].size.height);
}
-(void)windowDidResize:(NSNotification *)note{if(note.object==terminalWindow)[self fitTerminalText];}
-(void)appendTerminal:(NSString *)text color:(NSColor *)color{
 if(!text.length)return;
 NSDictionary *attributes=@{NSFontAttributeName:terminalInput.font,NSForegroundColorAttributeName:color};
 NSTextStorage *storage=terminalTranscript.textStorage;
 [storage appendAttributedString:[[NSAttributedString alloc]initWithString:text attributes:attributes]];
 if(storage.length>200000){
  NSRange cut=[storage.string rangeOfString:@"\n" options:0 range:NSMakeRange(16000,storage.length-16000)];
  if(cut.location!=NSNotFound)[storage deleteCharactersInRange:NSMakeRange(0,NSMaxRange(cut))];
 }
 [self fitTerminalText];[terminalWindow.contentView layoutSubtreeIfNeeded];
 [terminalTranscript scrollRangeToVisible:NSMakeRange(storage.length,0)];
}
-(void)showTerminal:(id)sender{
 if(skin!=LINUX_THEME || !p)return;
 if(!terminalWindow){
  NSRect work=window.screen.visibleFrame;
  if(NSIsEmptyRect(work))work=NSScreen.mainScreen.visibleFrame;
  CGFloat width=MIN(650,work.size.width-24),height=MIN(300,work.size.height-30);
  CGFloat x=MIN(NSMaxX(work)-width,MAX(NSMinX(work),NSMaxX(window.frame)-width+42));
  CGFloat y=MAX(NSMinY(work)+8,NSMinY(window.frame)-height+56);
  terminalWindow=[[NSWindow alloc]initWithContentRect:NSMakeRect(x,y,width,height)
   styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable|NSWindowStyleMaskResizable|NSWindowStyleMaskMiniaturizable
   backing:NSBackingStoreBuffered defer:NO];
  terminalWindow.releasedWhenClosed=NO;terminalWindow.title=@"RMP Terminal";
  terminalWindow.delegate=self;terminalWindow.minSize=NSMakeSize(400,180);
  terminalWindow.tabbingMode=NSWindowTabbingModeDisallowed;
  terminalWindow.collectionBehavior=NSWindowCollectionBehaviorMoveToActiveSpace|NSWindowCollectionBehaviorFullScreenAuxiliary;
  terminalWindow.appearance=[NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
  NSView *root=terminalWindow.contentView;
  NSColor *background=rgb(0x111416), *ink=rgb(0xdce2dd);
  terminalWindow.backgroundColor=background;root.wantsLayer=YES;root.layer.backgroundColor=background.CGColor;
  terminalScroll=[NSScrollView new];terminalScroll.borderType=NSNoBorder;terminalScroll.drawsBackground=NO;
  terminalScroll.hasVerticalScroller=YES;terminalScroll.autohidesScrollers=YES;
  terminalTranscript=[[NSTextView alloc]initWithFrame:NSZeroRect];terminalTranscript.editable=NO;terminalTranscript.selectable=YES;
  terminalTranscript.richText=NO;terminalTranscript.drawsBackground=NO;terminalTranscript.textContainerInset=NSZeroSize;
  terminalTranscript.textContainer.lineFragmentPadding=0;
  terminalTranscript.verticallyResizable=YES;terminalTranscript.horizontallyResizable=NO;
  terminalTranscript.autoresizingMask=NSViewWidthSizable;
  terminalTranscript.textContainer.widthTracksTextView=YES;
  terminalTranscript.maxSize=NSMakeSize(CGFLOAT_MAX,CGFLOAT_MAX);
  terminalScroll.documentView=terminalTranscript;
  NSFont *font=[NSFont monospacedSystemFontOfSize:13 weight:NSFontWeightRegular];
  terminalTranscript.font=font;
  terminalPrompt=[NSTextField labelWithString:@""];terminalPrompt.font=font;terminalPrompt.textColor=rgb(0x8bdc9e);
  terminalPrompt.lineBreakMode=NSLineBreakByTruncatingMiddle;
  terminalInput=[NSTextField new];terminalInput.font=font;terminalInput.textColor=ink;terminalInput.delegate=self;
  terminalInput.bezeled=NO;terminalInput.drawsBackground=NO;terminalInput.focusRingType=NSFocusRingTypeNone;
  [terminalInput setAccessibilityLabel:@"Terminal command"];
  terminalWindow.initialFirstResponder=terminalInput;
  for(NSView *view in @[terminalScroll,terminalPrompt,terminalInput]){
   [root addSubview:view];view.translatesAutoresizingMaskIntoConstraints=NO;
  }
  terminalTextHeight=[terminalScroll.heightAnchor constraintEqualToConstant:0];terminalTextHeight.priority=NSLayoutPriorityDefaultHigh;
  [NSLayoutConstraint activateConstraints:@[terminalTextHeight,
   [terminalScroll.leadingAnchor constraintEqualToAnchor:root.leadingAnchor constant:14],
   [terminalScroll.trailingAnchor constraintEqualToAnchor:root.trailingAnchor constant:-14],
   [terminalScroll.topAnchor constraintEqualToAnchor:root.topAnchor constant:12],
   [terminalScroll.bottomAnchor constraintEqualToAnchor:terminalInput.topAnchor constant:-3],
   [terminalInput.bottomAnchor constraintLessThanOrEqualToAnchor:root.bottomAnchor constant:-12],
   [terminalInput.trailingAnchor constraintEqualToAnchor:root.trailingAnchor constant:-14],
   [terminalInput.widthAnchor constraintGreaterThanOrEqualToConstant:100],
   [terminalPrompt.leadingAnchor constraintEqualToAnchor:root.leadingAnchor constant:14],
   [terminalPrompt.firstBaselineAnchor constraintEqualToAnchor:terminalInput.firstBaselineAnchor],
   [terminalInput.leadingAnchor constraintEqualToAnchor:terminalPrompt.trailingAnchor constant:6]
  ]];
  [root layoutSubtreeIfNeeded];
  terminalHistory=[NSMutableArray new];terminalPrompt.stringValue=takeTerminalString(ap_terminal_prompt(p));
  [self appendTerminal:@"RMP Terminal\nType help to list commands.\n" color:ink];
 }
 if(window.miniaturized)[window deminiaturize:nil];
 if(!window.visible)[window orderFront:nil];
 if(terminalWindow.miniaturized)[terminalWindow deminiaturize:nil];
 [terminalWindow makeKeyAndOrderFront:nil];[terminalWindow makeFirstResponder:terminalInput];
}
-(void)setTerminalLine:(NSString *)line{
 terminalInput.stringValue=line;
 NSText *editor=terminalInput.currentEditor;
 [editor setSelectedRange:NSMakeRange(line.length,0)];
}
-(void)submitTerminalLine{
 NSString *line=terminalInput.stringValue;
 if(!line.length)return;
 if(![terminalHistory.lastObject isEqual:line]){
  [terminalHistory addObject:line];if(terminalHistory.count>200)[terminalHistory removeObjectAtIndex:0];
 }
 terminalHistoryIndex=terminalHistory.count;terminalDraft=@"";
 [self appendTerminal:[terminalPrompt.stringValue stringByAppendingString:@" "] color:terminalPrompt.textColor];
 [self appendTerminal:[line stringByAppendingString:@"\n"] color:terminalInput.textColor];
 [self setTerminalLine:@""];
 int action=0;
 NSString *output=takeTerminalString(ap_terminal_command(p,line.UTF8String,&action));
 if(action==2){terminalTranscript.string=@"";[self fitTerminalText];}
 if(output.length)[self appendTerminal:[output hasSuffix:@"\n"]?output:[output stringByAppendingString:@"\n"] color:terminalInput.textColor];
 terminalPrompt.stringValue=takeTerminalString(ap_terminal_prompt(p));
 if(action==9 || (action>=10 && action<10+THEME_COUNT)){
  NSNumber *previous=[NSUserDefaults.standardUserDefaults objectForKey:@"PreviousTheme"];
  NSInteger choice=action==9?(previous?previous.integerValue:VLC_THEME):action-10;
  if(choice<0 || choice>=THEME_COUNT || (action==9 && choice==LINUX_THEME))choice=VLC_THEME;
  [self pickTheme:[themeMenu itemWithTag:choice]];
 }
 if(action==1){[window makeKeyAndOrderFront:nil];[self full:nil];}
 if(action==3 && ap_index(p)>=0)[self showPlayingItem:(NSUInteger)ap_index(p)];
 if(action==4){volume.intValue=ap_terminal_volume(p);if(fv)fv.intValue=volume.intValue;}
}
-(BOOL)control:(NSControl *)control textView:(NSTextView *)editor doCommandBySelector:(SEL)command{
 if(control!=terminalInput)return NO;
 if(command==@selector(insertNewline:)){[self submitTerminalLine];return YES;}
 if(command==@selector(insertTab:)){
  NSString *candidate=takeTerminalString(ap_terminal_complete(p,terminalInput.stringValue.UTF8String));
  if(candidate.length)[self setTerminalLine:candidate];
  return YES;
 }
 if(command==@selector(moveUp:)){
  if(terminalHistoryIndex==terminalHistory.count)terminalDraft=terminalInput.stringValue;
  if(terminalHistoryIndex>0)[self setTerminalLine:terminalHistory[--terminalHistoryIndex]];
  return YES;
 }
 if(command==@selector(moveDown:)){
  if(terminalHistoryIndex<terminalHistory.count)terminalHistoryIndex++;
  [self setTerminalLine:terminalHistoryIndex==terminalHistory.count?(terminalDraft?:@""):terminalHistory[terminalHistoryIndex]];
  return YES;
 }
 return NO;
}
-(void)windowDidBecomeKey:(NSNotification *)note{
 playMenuItem.keyEquivalent=note.object==terminalWindow?@"":@" ";
}
-(void)windowDidResignKey:(NSNotification *)note{
 if(note.object==terminalWindow)playMenuItem.keyEquivalent=@" ";
}
-(BOOL)windowShouldClose:(NSWindow *)closing{
 if(closing==terminalWindow){[terminalWindow orderOut:nil];[window makeKeyAndOrderFront:nil];return NO;}
 if(closing==window && terminalWindow){[terminalWindow orderOut:nil];[terminalWindow close];}
 return YES;
}
