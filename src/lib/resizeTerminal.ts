    import { Terminal } from "@xterm/xterm";
    import { FitAddon } from "@xterm/addon-fit";


export function incrementTerminalFont(term : Terminal, fitAddOn : FitAddon, sizeIncrement = 1, minFontSize = 6, maxFontSize = 40){
    if (sizeIncrement > 0 && term.options.fontSize && term.options.fontSize < maxFontSize) {
        term.options.fontSize += sizeIncrement;
        fitAddOn.fit();
    }
    if (sizeIncrement < 0 && term.options.fontSize && term.options.fontSize > minFontSize) {
        term.options.fontSize -= 1;
        fitAddOn.fit();
    }
}


export function setTerminalFont(term : Terminal, fitAddOn : FitAddon, fontSize : number, minFontSize = 6, maxFontSize = 40){
    if (term.options.fontSize && fontSize < maxFontSize && fontSize > minFontSize) {
        term.options.fontSize = fontSize;
        fitAddOn.fit();
    }
}