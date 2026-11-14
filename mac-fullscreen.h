-(void)windowDidEnterFullScreen:(NSNotification *)note {
 inFull=YES;[self arrangeChrome];
 panel=[[NSPanel alloc]initWithContentRect:NSMakeRect(0,0,550,76) styleMask:NSWindowStyleMaskBorderless|NSWindowStyleMaskNonactivatingPanel backing:NSBackingStoreBuffered defer:NO];
 panel.releasedWhenClosed=NO;panel.opaque=NO;panel.backgroundColor=NSColor.clearColor;
 panel.collectionBehavior=NSWindowCollectionBehaviorFullScreenAuxiliary;
 NSVisualEffectView *base=[NSVisualEffectView new];base.material=NSVisualEffectMaterialHUDWindow;base.state=NSVisualEffectStateActive;base.wantsLayer=YES;base.layer.cornerRadius=10;panel.contentView=base;
 fp=[self button:@"Play" action:@selector(toggle:)];fs=[NSSlider sliderWithValue:seek.doubleValue minValue:0 maxValue:1 target:self action:@selector(floatSeek:)];fs.continuous=NO;
 fv=[VolumeSlider sliderWithValue:volume.doubleValue minValue:0 maxValue:100 target:self action:@selector(floatVolume:)];
 NSStackView *buttons=[self row:@[[self button:@"Previous" action:@selector(prev:)],fp,[self button:@"Next" action:@selector(next:)],fv,[self button:@"Exit Full Screen" action:@selector(full:)]]];
 for(NSView *v in @[fs,buttons]){[base addSubview:v];v.translatesAutoresizingMaskIntoConstraints=NO;
 [v.leadingAnchor constraintEqualToAnchor:base.leadingAnchor constant:12].active=YES;[v.trailingAnchor constraintEqualToAnchor:base.trailingAnchor constant:-12].active=YES;}
 [fs.topAnchor constraintEqualToAnchor:base.topAnchor constant:6].active=YES;[buttons.bottomAnchor constraintEqualToAnchor:base.bottomAnchor constant:-6].active=YES;
 [self styleView:base];NSRect bounds=window.frame;[panel setFrameOrigin:NSMakePoint(NSMidX(bounds)-275,NSMinY(bounds)+28)];
 [window addChildWindow:panel ordered:NSWindowAbove];[panel orderFront:nil];lastPointer=NSEvent.mouseLocation;lastActivity=NSProcessInfo.processInfo.systemUptime;
}
-(void)windowWillExitFullScreen:(NSNotification *)note {
 inFull=NO;if(panel){[window removeChildWindow:panel];[panel orderOut:nil];[panel close];panel=nil;fp=nil;fs=nil;fv=nil;}[self arrangeChrome];
}
-(void)floatSeek:(NSSlider *)s {ap_seek(p,s.doubleValue*ap_length(p));}
-(void)floatVolume:(NSSlider *)s {volume.doubleValue=s.doubleValue;[self changeVolume:s];}
-(void)refreshFloating {
 if(!panel)return;fp.title=play.title;fs.enabled=seek.enabled;fs.doubleValue=seek.doubleValue;fv.doubleValue=volume.doubleValue;
 NSPoint point=NSEvent.mouseLocation;double now=NSProcessInfo.processInfo.systemUptime;
 if(!NSEqualPoints(point,lastPointer)||(panel.visible&&NSPointInRect(point,panel.frame))){
 lastActivity=now;if(!panel.visible)[panel orderFront:nil];
 }else if(now-lastActivity>2.5 && panel.visible)[panel orderOut:nil];
 lastPointer=point;
}
