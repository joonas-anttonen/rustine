project('rustine-wl', ['c', 'cpp'], default_options: ['cpp_std=c++17'])


wayland_client = dependency('wayland-client', required: true)

wayland_protocols = dependency('wayland-protocols', native: true, required: true)
wayland_protocol_dir = wayland_protocols.get_variable('pkgdatadir')

wayland_scanner = dependency('wayland-scanner', native: true, required: true)
wayland_scanner_prog = find_program(wayland_scanner.get_variable('wayland_scanner'), native: true)

wayland_scanner_code = generator(
	wayland_scanner_prog,
	output: '@BASENAME@-protocol.c',
	arguments: ['private-code', '@INPUT@', '@OUTPUT@'],
)

wayland_scanner_client = generator(
	wayland_scanner_prog,
	output: '@BASENAME@-client-protocol.h',
	arguments: ['client-header', '@INPUT@', '@OUTPUT@'],
)

client_protocols = [
	wayland_protocol_dir / 'stable/xdg-shell/xdg-shell.xml',
	wayland_protocol_dir / 'stable/viewporter/viewporter.xml',
	wayland_protocol_dir / 'staging/single-pixel-buffer/single-pixel-buffer-v1.xml',
	wayland_protocol_dir / 'staging/fractional-scale/fractional-scale-v1.xml',
	'wlr-layer-shell-unstable-v1.xml',
]

wayland_protocol_generated = []
foreach filename : client_protocols
	wayland_protocol_generated += wayland_scanner_code.process(filename)
	wayland_protocol_generated += wayland_scanner_client.process(filename)
endforeach

inc = include_directories('.')

static_library(
	'rustine-wl',
	sources: [
		'rustine-wl.cpp',
		wayland_protocol_generated,
	],
	include_directories: [inc],
	dependencies: [wayland_client],
	install: false,
)
