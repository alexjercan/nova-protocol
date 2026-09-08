#!/usr/bin/env python3
"""Render the four-page opening in the accepted frontal, clean-line treatment."""

import argparse
import math
import sys
from html import escape
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT/'scripts'))
sys.path.insert(0, str(OUTPUT.parent))
from nova_illustration.colors import INK, PAPER, CREAM, JADE, MINT, INTERIOR, SKY, SATURN, WORK, FOLIO, REVIEW
from nova_illustration.portraits import elena_close, elena_gesture, jonah_listener, work_portrait
from nova_illustration.scenery import stars, saturn, baikal
from nova_illustration.ships import render_ship
from nova_illustration.svg import ellipse, group, path, rect, tag, text
from opening_props import mug_hand


def definitions():
    """Supply the shared scenery's sky, planet lighting, and globe clip."""
    sky = tag('linearGradient',''.join(tag('stop',stop_color=SKY[k],offset=o) for k,o in [('top',0),('middle',.6),('bottom',1)]),id='space',x2='.8',y2='1')
    planet = tag('linearGradient',''.join(tag('stop',stop_color=SATURN[k],offset=o) for k,o in [('light',0),('middle',.58),('shadow',1)]),id='planet',x2='.9',y2='.6')
    return tag('defs',sky+planet+tag('clipPath',tag('circle',r=310),id='globe'))


def warm_room(w, h):
    """Place the exchange with cropped wall panels and a work surface, not an empty hall."""
    art = rect(0,0,w,h,INTERIOR['wall'])
    art += path(f'M0 0H{w*.58}L{w*.43} {h}H0Z',INTERIOR['wall_light'],'none')
    art += path(f'M{w*.84} 0H{w}V{h}H{w*.89}Z',INTERIOR['panel'],width=2)
    art += path(f'M{w*.87} 0V{h}M{w*.85} {h*.68}H{w}',stroke=INTERIOR['panel_line'],width=1.8)
    art += path(f'M0 {h*.84}L{w*.3} {h*.7}L{w*.67} {h}H0Z',INTERIOR['worktop'],width=2)
    art += path(f'M0 {h*.86}L{w*.3} {h*.72}L{w*.48} {h*.87}',stroke=INTERIOR['edge'],width=2)
    return art


def machinery_wall(w, h):
    """Use sparse service pipes as anchors behind the people and stopped pump."""
    art = rect(0,0,w,h,WORK['wall'])
    art += path(f'M0 0H{w*.36}L{w*.18} {h}H0Z',WORK['wall_shadow'],'none')
    for x in (48,w*.38,w*.88):
        art += path(f'M{x} 0V{h}',stroke=WORK['wall_shadow'],width=25)
        art += path(f'M{x-6} 0V{h}',stroke=WORK['pipe'],width=6)
    art += path(f'M0 {h*.65}H{w}',stroke=WORK['wall_shadow'],width=12)
    return art


def pump(x, y, scale):
    """Return rear pipes and a foreground housing for this scene's painter order."""
    art = path('M-320-30H-90V-205H248',stroke=INK,width=96)
    art += path('M-320-30H-90V-205H248',stroke=WORK['pipe'],width=80)
    art += path('M-320-50H-109V-219H248',stroke=WORK['pipe_light'],width=8)
    pipes = art
    art = rect(-178,111,350,39,WORK['wall_shadow'],INK,3)
    art += ellipse(28,0,195,158,WORK['wall_shadow'],INK,3)
    art += ellipse(0,0,195,158,WORK['pump'],INK,3)
    art += ellipse(0,0,156,127,WORK['pump_light'],INK,2.5)
    art += ellipse(0,0,114,92,WORK['wall_shadow'],INK,2.5)
    art += ellipse(0,0,88,72,WORK['pipe'],INK,2)
    for i in range(10):
        a = math.tau*i/10
        art += ellipse(math.cos(a)*177,math.sin(a)*143,5,5,INK,WORK['pipe_light'],1.5)
    art += path('M-166 132H160',stroke=WORK['warning'],width=6,stroke_dasharray='12 12')
    art += rect(-68,-48,136,96,WORK['screen'],INK,2,6)
    art += path('M-47-22H46M-47 0H15M-47 22H35',stroke=WORK['pipe_light'],width=3)
    transform = f'translate({x} {y}) scale({scale})'
    return group(pipes,transform),group(art,transform)


class Panel:
    """A clipped scene with separate dialogue and orientation-card layers."""

    def __init__(self, key, x, y, w, h, title, art):
        self.key, self.x, self.y, self.w, self.h = key,x,y,w,h
        self.title, self.art, self.lines, self.lettering = title,art,[],''

    def say(self, x, y, w, speaker, lines, tip):
        """Keep explicit wrapping and speaker labels, including comms and off-panel voices."""
        h = 42+27*len(lines)
        tx,ty = tip
        anchor = max(x+25,min(x+w-30,tx))
        dx,dy = tx-anchor,ty-y-h
        length = math.hypot(dx,dy)
        if length > 65:
            tx,ty = anchor+dx*65/length,y+h+dy*65/length
        shape = path(f'M{x+24} {y}H{x+w-24}Q{x+w} {y} {x+w} {y+24}V{y+h-24}Q{x+w} {y+h} {x+w-24} {y+h}H{anchor+18}L{tx} {ty}L{anchor-5} {y+h}H{x+24}Q{x} {y+h} {x} {y+h-24}V{y+24}Q{x} {y} {x+24} {y}Z',CREAM,width=2.5)
        shape += text(x+18,y+25,speaker.upper(),13,FOLIO['title'],font_weight='bold',letter_spacing=1.3)
        shape += ''.join(text(x+18,y+52+i*27,line,22,INK) for i,line in enumerate(lines))
        self.lettering += group(shape,class_='dialogue',data_x=x,data_y=y,data_width=w,data_height=h)
        self.lines.append((speaker,' '.join(lines)))
        return self

    def title_card(self, x, y, w, name, place, date, note):
        """Retain the established orientation text and its explicit provisional date."""
        art = rect(x,y,w,137,WORK['screen'],WORK['card_border'],1,2,opacity=.97)
        art += rect(x,y,5,137,MINT)
        art += text(x+21,y+34,name,26,WORK['card_title'],font_weight='bold',letter_spacing=2.5)
        art += text(x+21,y+62,place,16,MINT)
        art += text(x+21,y+89,date,13,WORK['warning'],letter_spacing=.4)
        art += text(x+21,y+116,note,15,WORK['card_text'])
        self.lettering += group(art,class_='title-card')
        self.lines.append(('Location and time',f'{name}. {place}. {date}. {note}'))
        return self

    def render(self):
        """Keep page frames and text outside scene art; clip without a grain overlay."""
        clip = tag('defs',tag('clipPath',rect(0,0,self.w,self.h,'white'),id=f'clip-{self.key}'))
        art = group(self.art,class_='scene-art')+self.lettering
        return group(clip+group(art,clip_path=f'url(#clip-{self.key})')+rect(0,0,self.w,self.h,'none',INK,2.5),f'translate({self.x} {self.y})',role='group',aria_label=self.title)


def pages():
    """Retain the four-page sequence and dialogue while restaging its ten panels."""
    w = 1416
    art = rect(0,0,w,360,'url(#space)')+stars(w,360)+saturn(1210,-83,.9)
    art += baikal(644,227,.63)+render_ship('ebro','forward-quarter',845,304,.23,False)+render_ship('kaveri','forward-quarter',440,308,.19,False)
    a = Panel('1a',42,86,w,360,'Baikal at work, with Ebro and Kaveri berthed against Saturn.',art)
    art = warm_room(w,477)
    art += rect(50,210,204,114,WORK['screen'],INK,2,4)+text(72,249,'GAME NIGHT',19,MINT,letter_spacing=1)+path('M72 270H214M72 290H163',stroke=INTERIOR['edge'],width=3)
    art += group(work_portrait('rina'),'translate(470 161) scale(.87)')
    art += group(jonah_listener(),'translate(952 161) scale(.87)')+mug_hand(942,421,.87)
    b = Panel('1b',42,466,w,477,'Rina and Jonah share a coffee break in Baikal before Leila calls from processing.',art)
    b.title_card(25,24,365,'BAIKAL','Saturn system','Opening day / calendar date TBD',"Clearwell Waterworks' main base")
    b.say(426,20,500,'Rina',["Coffee's still hot. Take a minute",'before somebody finds you','another job.'],(620,215))
    b.say(982,20,409,'Jonah',["I'll settle for half a minute."],(1128,214))
    b.say(25,320,365,'Leila / comms',["Jonah? Processing line's",'stopped. Can you','come down?'],(12,472))
    page1 = ('A place to come back to','An inhabited workplace, a coffee break, and an ordinary call from processing.',[a,b])

    art = machinery_wall(836,857)
    pipes,housing = pump(174,639,1.36)
    art += pipes+group(work_portrait('leila'),'translate(362 185) scale(1.06)')
    art += housing+group(jonah_listener(),'translate(555 474) scale(1.05)')
    a = Panel('2a',42,86,836,857,'Leila and Jonah assess the safely isolated circulation assembly, close to the machinery.',art)
    a.say(30,24,513,'Leila',["Circulation pump's failed.","We've isolated the line.",'The others are still running.'],(548,286))
    art = machinery_wall(560,364)+rect(38,29,484,306,WORK['screen'],INK,4,13)
    art += rect(62,56,436,119,WORK['warning_back'],WORK['pump'],2,5)+text(83,89,'INDUSTRIAL PROCESSING',17,WORK['warning'],letter_spacing=1)+text(83,129,'LINE ISOLATED',29,WORK['warning'],font_weight='bold')
    art += rect(62,190,436,118,WORK['safe_back'],JADE,2,5)+text(83,224,'RESIDENTIAL SUPPLIES',17,WORK['safe'],letter_spacing=1)+text(83,266,'NORMAL',30,WORK['safe'],font_weight='bold')
    b = Panel('2b',898,86,560,364,'The industrial line is stopped; residential supplies remain normal.',art)
    b.lines.extend([('Display','Industrial processing: line isolated.'),('Display','Residential supplies: normal.')])
    c = Panel('2c',898,470,560,473,'Leila explains why replacing the assembly is quicker than an overhaul.',machinery_wall(560,473)+group(work_portrait('leila'),'translate(265 164) scale(1.02)'))
    c.say(18,18,314,'Jonah / off-panel',['Can you rebuild it here?'],(3,158))
    c.say(18,144,314,'Leila',["Yes. But we'd lose",'more production waiting','on the overhaul.','A replacement gets us','running sooner.'],(403,317))
    page2 = ('One line down','A repair judgment, not a residential life-support crisis.',[a,b,c])

    art = warm_room(w,415)
    view = rect(425,16,990,337,'url(#space)')+stars(w,353)
    view += path('M440 349L714 282L759 259',stroke=INTERIOR['structure'],width=18)
    view += path('M444 344L714 276L759 254',stroke=INTERIOR['structure_light'],width=4)
    view += render_ship('ebro','forward-quarter',846,254,.64,False)
    art += tag('defs',tag('clipPath',rect(425,16,990,337,'white',radius=72),id='window'))+group(view,clip_path='url(#window)')
    art += rect(419,10,1002,349,'none',INK,21,78)+rect(421,12,998,345,'none',INTERIOR['window_frame'],13,76)
    art += path('M476 322V110Q476 61 515 61M1378 19V350',stroke=INTERIOR['window_light'],width=3)
    art += path('M410 372H1416',stroke=INTERIOR['panel'],width=19)
    art += group(elena_gesture(),'translate(163 146) scale(.64)')+group(jonah_listener(),'translate(1067 136) scale(.91)')
    a = Panel('3a',42,86,w,415,'Elena and Jonah arrange the Aquila pickup, with the waiting Ebro framed by the window.',art)
    a.say(25,20,430,'Elena',['Aquila has a replacement','assembly. Can Kaveri','collect it?'],(270,245))
    a.say(958,20,433,'Jonah',['Yes. We can leave as soon',"as everyone's aboard."],(1237,210))
    art = warm_room(680,422)+group(elena_close(),'translate(340 126) scale(.94)')
    record = rect(-120,-65,240,153,WORK['screen'],INK,3,7)+text(-98,-25,'EBRO',29,MINT,font_weight='bold')+text(-98,11,'WATER DELIVERY',17,INTERIOR['edge'])+text(-98,52,'FOUNDATION',21,WORK['warning'])
    art += group(record,'translate(164 312) rotate(-8)')
    b = Panel('3b',42,521,680,422,'A delivery record connects Ebro and Foundation while Elena explains the commitment.',art)
    b.say(20,20,322,'Elena',['EarthWorks is expecting',"Ebro's load aboard",'Foundation. We need that','line back to have it','ready on time.'],(463,300))
    b.lines.append(('Delivery record','Ebro. Water delivery for Foundation.'))
    art = warm_room(716,422)+group(elena_close(),'translate(322 131)')+mug_hand(177,340,.9)
    c = Panel('3c',742,521,716,422,'Elena closes the exchange with the borrowed-mug joke; Jonah answers from the foreground.',art)
    c.say(22,20,352,'Elena',['And bring the mug back.',"We haven't budgeted",'for a replacement.'],(463,253))
    c.say(24,177,247,'Jonah',["I'll put it on",'the cargo list.'],(0,340))
    page3 = ('A useful job','The same pickup, delivery commitment, and borrowed-mug exchange, framed closer.',[a,b,c])

    art = rect(0,0,w,407,WORK['wall_shadow'])+rect(113,-35,1190,352,'url(#space)',INK,18,60)+stars(w,300)+baikal(522,159,.29)
    art += path('M107 0L207 326H1220L1324 0M874 0L852 326',stroke=INTERIOR['structure_light'],width=12)
    art += group(work_portrait('tomas'),'translate(482 147) scale(.74)')+group(jonah_listener(),'translate(1034 141) scale(.86)')
    art += path('M0 407L192 368H551L876 407Z',WORK['screen'],width=3)
    art += path('M244 378H455L478 400H222Z',INTERIOR['panel'],width=2)+path('M268 389H421M507 389H549M580 389H632',stroke=MINT,width=3)
    a = Panel('4a',42,86,w,407,'A Kaveri location card introduces the move aboard. Tomas reports readiness and Jonah gives the departure order.',art)
    a.title_card(25,20,390,'KAVERI','Departing Baikal','Later that day / calendar date TBD',"Clearwell Waterworks' workship")
    a.say(455,20,472,'Tomas',['All aboard. Cargo space is','clear, and the Aquila course','is ready.'],(636,215))
    a.say(999,20,389,'Jonah',['All right. Take us out.'],(1197,210))
    art = rect(0,0,w,430,'url(#space)')+stars(w,430)+saturn(1260,-158,1.2)+baikal(285,197,.46)
    art += render_ship('ebro','forward-quarter',421,257,.16,False)+render_ship('kaveri','aft-quarter',1031,279,1.15,True)
    b = Panel('4b',42,513,w,430,'Kaveri departs with its cradle empty and handling arm stowed. Baikal and Ebro recede. No attack or pursuit appears.',art)
    page4 = ('Outbound','A silent departure. Aquila is the next destination, not an arrival shown here.',[a,b])
    return [page1,page2,page3,page4]


def render_page(number, title, panels):
    """Render a complete page with unique IDs and the accepted warm-paper frame."""
    art = definitions()+rect(0,0,1500,1000,PAPER)
    art += text(43,48,'NOVA PROTOCOL',21,FOLIO['title'],font_weight='bold',letter_spacing=3.5)
    art += text(412,48,'OPENING / CLEAN-LINE DRAFT',13,FOLIO['muted'],letter_spacing=2)
    art += text(1456,48,f'{number:02}',24,INK,text_anchor='end',font_weight='bold')
    art += ''.join(panel.render() for panel in panels)
    art += text(43,977,'PRIVATE COMIC DRAFT / DESIGNS AND DIALOGUE PROVISIONAL',12,FOLIO['muted'],letter_spacing=1.2)
    art += text(1456,977,title.upper(),12,FOLIO['title'],text_anchor='end',letter_spacing=1.5)
    art = art.replace('id="',f'id="p{number}-').replace('url(#',f'url(#p{number}-')
    label = f'Page {number}: {title}. '+' '.join(panel.title for panel in panels)
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="1500" height="1000" viewBox="0 0 1500 1000" role="img" aria-labelledby="page-{number}-title page-{number}-description"><title id="page-{number}-title">{escape(title)}</title><desc id="page-{number}-description">{escape(label)}</desc>{art}</svg>\n'


def render_review(authored, svgs):
    """Keep the private four-page board, its controls, and text transcripts."""
    cards = []
    for number,((title,purpose,panels),svg) in enumerate(zip(authored,svgs),1):
        transcript = ''.join(f'<li><strong>{escape(speaker)}:</strong> {escape(line)}</li>' for panel in panels for speaker,line in panel.lines)
        hidden = ' hidden' if number>1 else ''
        cards.append(f'<section class="page" id="page-{number}" aria-labelledby="heading-{number}"{hidden}><div class="page-heading"><h2 id="heading-{number}">{number:02} {escape(title)}</h2><a href="page-{number:02}.svg">Full-size SVG</a></div>{svg}<div class="page-notes"><p>{escape(purpose)}</p><details><summary>Read dialogue as text</summary><ol>{transcript}</ol><p>{escape(panels[-1].title) if not panels[-1].lines else ""}</p></details></div></section>')
    navigation = ''.join(f'<a href="#page-{n}" data-number="{n}" aria-label="Page {n}: {escape(title)}">{n:02}</a>' for n,(title,_,_) in enumerate(authored,1))
    theme = ';'.join(f'--{key.replace("_","-")}:{value}' for key,value in REVIEW.items())
    return '''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Baikal / Opening comic draft</title><style>
:root{'''+theme+''';color-scheme:dark}*{box-sizing:border-box}body{margin:0;background:var(--background);color:var(--text);font:16px/1.6 system-ui,sans-serif}a{color:var(--mint)}a:focus-visible,button:focus-visible,summary:focus-visible{outline:3px solid var(--focus);outline-offset:4px}header,.control-inner,main,footer{max-width:1500px;margin:auto;padding:24px 32px}header{padding-bottom:12px}h1{font-size:clamp(26px,4vw,40px);line-height:1.2;font-weight:500;margin:8px 0 16px}.eyebrow{color:var(--mint);font-size:11px;letter-spacing:.2em;font-weight:700}p{max-width:85ch;color:var(--paragraph)}.controls{background:var(--surface);position:sticky;top:0;z-index:2;border-block:1px solid var(--border)}.control-inner{display:flex;gap:10px;align-items:center;flex-wrap:wrap;padding-block:10px}nav{display:flex;gap:5px;margin-right:auto}nav a,button{font:600 12px/1.2 system-ui,sans-serif;background:var(--button);border:1px solid var(--border);border-radius:4px;color:var(--text);min-height:38px;padding:10px 14px;text-decoration:none;cursor:pointer}nav a[aria-current],button[aria-pressed="true"]{background:var(--mint);color:var(--button-text)}button:disabled{opacity:.3;cursor:default}.page-heading{display:flex;justify-content:space-between;align-items:baseline;gap:16px;margin-bottom:12px}h2{font-size:21px;font-weight:500;line-height:1.25;margin:0}.page-heading a{font-size:13px;white-space:nowrap}.page>svg{display:block;width:100%;height:auto}.page-notes{padding:8px 0 24px;font-size:14px}details{background:var(--surface);padding:12px 18px;border-radius:4px}summary{cursor:pointer}li{margin:8px 0}ol{font-size:16px;max-width:70ch}.art-only .dialogue{visibility:hidden}.contact main{display:grid;grid-template-columns:1fr 1fr;gap:25px}.contact .page-heading a,.contact details{display:none}footer{font-size:13px;border-top:1px solid var(--border)}#counter{font-size:12px;color:var(--paragraph)}@media(max-width:700px){header,.control-inner,main,footer{padding-left:16px;padding-right:16px}.contact main{grid-template-columns:1fr}.control-inner{gap:6px}button,nav a{padding:9px 11px}h2{font-size:18px}.page-heading a{font-size:11px}#counter{display:none}}@media print{header,.controls,footer,.page-heading,.page-notes{display:none!important}main,.contact main{display:block;padding:0;max-width:none}.page,.page[hidden]{display:block!important;break-after:page}@page{size:landscape;margin:0}}
</style></head><body><header><p class="eyebrow">NOVA PROTOCOL / PRIVATE COMIC DRAFT</p><h1>Baikal, before the trouble.</h1><p>The same four opening pages in the accepted style: frontal faces, closer framing, and shared industrial ships.</p><p><a href="../proof/before-clean-line-opening/index.html">Compare the previous four-page version</a> / <a href="../clean-line-study/index.html">Accepted three-shot study</a></p></header><div class="controls"><div class="control-inner"><nav aria-label="Draft pages">'''+navigation+'''</nav><span id="counter" aria-live="polite">Page 1 of 4</span><button id="previous" type="button" disabled>Previous</button><button id="next" type="button">Next</button><button id="contact" type="button" aria-pressed="false">Contact sheet</button><button id="art" type="button" aria-pressed="false">Art only</button></div></div><main>'''+''.join(cards)+'''</main><footer><p>Working designs and dialogue, not released story or an episode boundary. No calendar year or nearby moon has been chosen. Use full-size SVGs or transcripts on a small screen.</p><p><a href="README.md">Scope and regeneration</a></p></footer><script>
const pages=[...document.querySelectorAll('.page')],links=[...document.querySelectorAll('[data-number]')],previous=document.getElementById('previous'),next=document.getElementById('next'),contact=document.getElementById('contact'),art=document.getElementById('art');let current=1,sheet=false;
function show(value){current=Math.max(1,Math.min(4,value));pages.forEach((page,i)=>page.hidden=!sheet&&i!==current-1);links.forEach((link,i)=>{if(i===current-1)link.setAttribute('aria-current','page');else link.removeAttribute('aria-current')});previous.disabled=current===1;next.disabled=current===4;document.getElementById('counter').textContent=`Page ${current} of 4`}
function fromHash(){const match=location.hash.match(/^#page-([1-4])$/);show(match?Number(match[1]):1)}
function step(delta){location.hash=`page-${Math.max(1,Math.min(4,current+delta))}`}
previous.addEventListener('click',()=>step(-1));next.addEventListener('click',()=>step(1));window.addEventListener('hashchange',fromHash);
contact.addEventListener('click',()=>{sheet=!sheet;document.body.classList.toggle('contact',sheet);contact.setAttribute('aria-pressed',String(sheet));show(current)});
art.addEventListener('click',()=>{const active=document.body.classList.toggle('art-only');art.setAttribute('aria-pressed',String(active))});
window.addEventListener('keydown',event=>{if(event.target.closest('button,a,summary,input,textarea,select')||event.altKey||event.ctrlKey||event.metaKey)return;if(event.key==='ArrowRight'){event.preventDefault();step(1)}if(event.key==='ArrowLeft'){event.preventDefault();step(-1)}});fromHash();
</script></body></html>
'''


def main():
    """Write only this private board's four SVGs and HTML, or verify their saved bytes."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args = parser.parse_args()
    authored = pages()
    svgs = [render_page(n,title,panels) for n,(title,_,panels) in enumerate(authored,1)]
    outputs = {f'page-{n:02}.svg':svg for n,svg in enumerate(svgs,1)}
    outputs['index.html'] = render_review(authored,svgs)
    for filename,content in outputs.items():
        destination = OUTPUT/filename
        encoded = content.encode('utf-8')
        if args.check:
            if not destination.exists() or destination.read_bytes()!=encoded:
                raise SystemExit(f'Out of date: {destination}')
        else:
            destination.write_bytes(encoded)
    print(f"{'Verified' if args.check else 'Rendered'} four clean-line pages and the private review board")


if __name__=='__main__':
    main()
