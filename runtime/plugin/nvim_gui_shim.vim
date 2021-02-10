" A Neovim plugin that implements GUI helper commands
if !has('nvim') || exists('g:GuiLoaded')
	finish
endif
let g:GuiLoaded = 1

function s:GuiClipboard()
    let g:clipboard = {
            \ 'name': 'gtk',
            \ 'copy': {
            \     '+': {lines, regtype -> rpcnotify(1, 'Gui', 'Clipboard', 'Set', '+', join(lines, "\n"))},
            \     '*': {lines, regtype -> rpcnotify(1, 'Gui', 'Clipboard', 'Set', '*', join(lines, "\n"))},
            \ },
            \ 'paste': {
            \     '+': { -> rpcrequest(1, 'Gui', 'Clipboard', 'Get', '+') },
            \     '*': { -> rpcrequest(1, 'Gui', 'Clipboard', 'Get', '*') },
            \ },
            \ 'cache_enabled': 0,
        \ }
    " borrowed from neovim-qt
    unlet! g:loaded_clipboard_provider
    runtime autoload/provider/clipboard.vim
endfunction

" Set GUI font
function! GuiFont(fname, ...) abort
	call rpcnotify(1, 'Gui', 'Font', s:NvimQtToPangoFont(a:fname))
endfunction

" Some subset of parse command from neovim-qt
" to support interoperability
function s:NvimQtToPangoFont(fname)
	let l:attrs = split(a:fname, ':')
	let l:size = -1
	for part in l:attrs
		if len(part) >= 2 && part[0] == 'h'
			let l:size = strpart(part, 1)
		endif
	endfor

	if l:size > 0
		return l:attrs[0] . ' ' . l:size
	endif

	return l:attrs[0]
endf


" The GuiFont command. For compatibility there is also Guifont
function s:GuiFontCommand(fname, bang) abort
	if a:fname ==# ''
		if exists('g:GuiFont')
			echo g:GuiFont
		else
			echo 'No GuiFont is set'
		endif
	else
		call GuiFont(a:fname, a:bang ==# '!')
	endif
endfunction
command! -nargs=1 -bang Guifont call s:GuiFontCommand("<args>", "<bang>")
command! -nargs=1 -bang GuiFont call s:GuiFontCommand("<args>", "<bang>")

command! -nargs=? GuiFontFeatures call rpcnotify(1, 'Gui', 'FontFeatures', <q-args>)
command! -nargs=1 GuiLinespace call rpcnotify(1, 'Gui', 'Linespace', <q-args>)

command! NGClipboard call s:GuiClipboard()

command! NGToggleSidebar call rpcnotify(1, 'Gui', 'Command', 'ToggleSidebar')
command! NGOpenSidebar call rpcnotify(1, 'Gui', 'Command', 'OpenSidebar')
command! NGCloseSidebar call rpcnotify(1, 'Gui', 'Command', 'CloseSidebar')
command! NGShowProjectView call rpcnotify(1, 'Gui', 'Command', 'ShowProjectView')
command! -nargs=+ NGTransparency call rpcnotify(1, 'Gui', 'Command', 'Transparency', <f-args>)
command! -nargs=1 NGPreferDarkTheme call rpcnotify(1, 'Gui', 'Command', 'PreferDarkTheme', <q-args>)
command! -nargs=1 NGSetCursorBlink call rpcnotify(1, 'Gui', 'Command', 'SetCursorBlink', <q-args>)
command! NGToggleFullscreen call rpcnotify(1, 'Gui', 'Command', 'ToggleFullscreen')
command! NGFullscreen call rpcnotify(1, 'Gui', 'Command', 'Fullscreen')
command! NGUnfullscreen call rpcnotify(1, 'Gui', 'Command', 'Unfullscreen')
command! NGHideExtTabline call rpcnotify(1, 'Gui', 'Command', 'HideExtTabline')
command! NGUnhideExtTabline call rpcnotify(1, 'Gui', 'Command', 'UnhideExtTabline')
command! NGSidebarShowLines call rpcnotify(1, 'Gui', 'Command', 'SidebarShowLines')
command! NGSidebarHideLines call rpcnotify(1, 'Gui', 'Command', 'SidebarHideLines')
command! NGSidebarToggleLines call rpcnotify(1, 'Gui', 'Command', 'SidebarToggleLines')
command! NGSidebarShowHidden call rpcnotify(1, 'Gui', 'Command', 'SidebarShowHidden')
command! NGSidebarHideHidden call rpcnotify(1, 'Gui', 'Command', 'SidebarHideHidden')
command! NGSidebarToggleHidden call rpcnotify(1, 'Gui', 'Command', 'SidebarToggleHidden')
command! NGTogglePreview call rpcnotify(1, 'Gui', 'Command', 'TogglePreview')
command! NGShowPreview call rpcnotify(1, 'Gui', 'Command', 'ShowPreview')
command! NGHidePreview call rpcnotify(1, 'Gui', 'Command', 'HidePreview')
command! -nargs=1 NGSetPreviewType call rpcnotify(1, 'Gui', 'Command', 'SetPreviewType', <q-args>)
command! -nargs=1 NGSetPreviewWidth call rpcnotify(1, 'Gui', 'Command', 'SetPreviewWidth', <q-args>)

" autocmds
autocmd BufEnter * call rpcnotify(1, 'Gui', 'Command', 'SetPreviewType', &filetype)
