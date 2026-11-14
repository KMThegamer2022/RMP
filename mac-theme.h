#import "themes.h"
static NSColor *rgb(unsigned v) {
 return [NSColor colorWithSRGBRed:(v>>16)/255.0 green:((v>>8)&255)/255.0 blue:(v&255)/255.0 alpha:1];
}
@interface ThemeSurface:NSView
@property NSInteger skin;
@end
@implementation ThemeSurface
-(void)drawRect:(NSRect)dirty {
 Theme t=themes[self.skin];
 if(self.skin==SEVEN){
 NSGradient *g=[[NSGradient alloc]initWithColors:@[rgb(t.panel),rgb(0xf2faff),rgb(0xa2c6e3),rgb(t.panel)]];
 [g drawInRect:self.bounds angle:90];
 }else{[rgb(t.panel)setFill];NSRectFill(dirty);}
 if(self.skin==XP || self.skin==MODERN){
 NSRect header=self.bounds;header.origin.y=NSMaxY(header)-58;header.size.height=58;
 if(self.skin==XP){
 NSGradient *g=[[NSGradient alloc]initWithColors:@[rgb(0x1244b5),rgb(0x1260e9),rgb(0x0058e5),rgb(0x4796ff)]];
 [g drawInRect:header angle:90];
 }else{[rgb(0xf5f5f5)setFill];NSRectFill(header);}
 }
}
@end
@interface TransportButton:NSButton
@property NSInteger skin;
@end
@implementation TransportButton
-(NSSize)intrinsicContentSize {NSSize s=super.intrinsicContentSize;s.width+=14;s.height=30;return s;}
-(void)drawRect:(NSRect)dirty {
 Theme t=themes[self.skin];NSRect r=NSInsetRect(self.bounds,1.5,2);
 BOOL play=self.action==@selector(toggle:),prev=self.action==@selector(prev:),next=self.action==@selector(next:);
 BOOL transport=play||prev||next,orb=self.skin==SEVEN && play;
 if(orb){CGFloat d=MIN(r.size.width,r.size.height);r=NSMakeRect(NSMidX(r)-d/2,NSMidY(r)-d/2,d,d);}
 NSBezierPath *path=[NSBezierPath bezierPathWithRoundedRect:r xRadius:orb?NSHeight(r)/2:t.radius yRadius:orb?NSHeight(r)/2:t.radius];
 BOOL pressed=self.cell.isHighlighted;
 [NSGraphicsContext saveGraphicsState];[path addClip];
 if(self.skin==XP || self.skin==SEVEN){
 NSArray *stops=orb?@[rgb(0x103369),rgb(0x2585df),rgb(0x1953ae),rgb(0xd8f7ff)]:@[rgb(pressed?t.edge:t.face),rgb(t.face),rgb(t.highlight)];
 NSGradient *g=[[NSGradient alloc]initWithColors:stops];
 [g drawInRect:r angle:90];
 }else{[rgb(pressed?t.edge:t.face)setFill];[path fill];}
 [NSGraphicsContext restoreGraphicsState];[rgb(t.edge)setStroke];if(self.skin!=MODERN)[path stroke];
 if(self.skin==XP && !pressed){[rgb(0xffffff)setStroke];[[NSBezierPath bezierPathWithRoundedRect:NSInsetRect(r,1,1) xRadius:2 yRadius:2]stroke];}
 NSDictionary *attrs=@{NSFontAttributeName:self.font,NSForegroundColorAttributeName:rgb(t.text)};
 NSString *caption=self.title;
 if(transport && self.skin!=MAC){
 caption=play?([self.title isEqualToString:@"Pause"]?@"Ⅱ":@"▶"):(prev?@"❘◀":@"▶❘");
 attrs=@{NSFontAttributeName:[NSFont systemFontOfSize:play?17:13],NSForegroundColorAttributeName:rgb(orb?0xffffff:t.text)};
 }
 NSSize size=[caption sizeWithAttributes:attrs];
 [caption drawAtPoint:NSMakePoint(NSMidX(r)-size.width/2,NSMidY(r)-size.height/2) withAttributes:attrs];
 if(self.window.firstResponder==self){[rgb(t.accent)setStroke];path.lineWidth=2;[path stroke];}
}
@end
