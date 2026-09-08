"""The illustration palette. These presentation colors do not establish physical livery."""

from types import MappingProxyType

INK = '#23333c'
PAPER = '#f4ecd9'
CREAM = '#fff4dd'
JADE = '#508c79'
MINT = '#a7d3b4'

ELENA = MappingProxyType({
    'skin': '#b98b72', 'shadow': '#79534a', 'light': '#edc3a1',
    'lines': '#79534a', 'ink': '#342d3b',
    'hair': '#484451', 'hair_light': '#c5b9a4', 'hair_line': '#888e87',
    'brow': '#534147', 'iris': '#526250', 'eye': '#eee1c6',
    'coat': JADE, 'coat_shadow': '#366456', 'coat_line': '#2b5149',
    'coat_light': '#a1c5a6', 'shirt': '#dbcdb0', 'shirt_shadow': '#294841',
})
JONAH = MappingProxyType({
    'skin': '#b58e72', 'shadow': '#695346', 'light': '#dec0a1',
    'lines': '#725645', 'ink': '#745d4e', 'stubble': '#8d7863',
    'hair': '#39332f', 'hair_line': '#687375', 'brow': '#43392e',
    'iris': '#30352f', 'eye': '#c5bda5', 'detail': '#997356', 'coat': '#56798d',
    'coat_shadow': '#3b566b', 'coat_light': '#8fa8ac', 'shirt': '#a8b1a9',
})
LEILA = MappingProxyType({
    'skin': '#b39577', 'shadow': '#6f5746', 'light': '#dfc5a1',
    'hair': '#35312d', 'hair_line': '#807665', 'brow': '#514030',
    'eye': '#d2c5ab', 'iris': '#3b4137', 'lines': '#785c49', 'detail': '#99785a',
    'coat': '#728662', 'coat_shadow': '#4c6249', 'coat_light': '#aec19b',
    'shirt': '#b1ab8f',
})
RINA = MappingProxyType({
    'skin': '#8b6755', 'shadow': '#4d3c33', 'light': '#c09a78',
    'hair': '#292b2a', 'hair_line': '#625b4e', 'brow': '#41322a',
    'eye': '#c6b799', 'iris': '#2c342e', 'lines': '#554035', 'detail': '#b38a69',
    'lip': '#795346', 'ink': '#4c3830', 'coat': '#ca8758',
    'coat_shadow': '#956347', 'coat_light': '#e5b685', 'shirt': '#b8ab86',
})
TOMAS = MappingProxyType({
    'skin': '#be9c83', 'shadow': '#715a4b', 'light': '#e0c5a9',
    'hair': '#3e3e3a', 'hair_line': '#a3a398', 'brow': '#4a4137',
    'eye': '#cabfa7', 'iris': '#36443d', 'lines': '#785c48', 'detail': '#a48165',
    'coat': '#395c78', 'coat_shadow': '#293f56', 'coat_light': '#86a4b4',
    'shirt': '#87928d',
})
MATERIALS = MappingProxyType({
    'paint': '#b4b9b5', 'metal': '#84979b', 'jade': '#5c8e79',
    'tank': '#c7c7b5', 'shadow': '#52616c', 'dark': '#29313b',
    'plate': '#a7afb4', 'plate_light': '#cbd0c9', 'steel': '#676c70',
    'hazard': '#e0b352',
    'accent': '#c4a277', 'glass': '#7daaa7', 'mint': '#b7dfbd',
    'ceramic': '#ead8b5', 'ceramic_shadow': '#bdad8b',
    'ceramic_highlight': '#f6e7ca', 'ceramic_edge': '#dcc8a5',
    'dark_brown': '#645045',
})
SHIP_INK = '#293e43'
REGISTRY_PAINT = '#c9dcc0'
EXHAUST = '#98d6cb'

STATION = MappingProxyType({
    'frame': '#698e86', 'frame_light': '#a9bca4', 'shadow': '#294755',
    'tank': '#b4c0a6', 'tank_shadow': '#659b88', 'paint': '#e2d6ad',
    'green': '#4b786d', 'ring': '#c2c7aa', 'ring_shadow': '#416575',
    'accent': '#cfaa78', 'solar': '#3c5773', 'solar_line': '#91a5aa',
})
SATURN = MappingProxyType({
    'light': '#f4d9a0', 'middle': '#d3ae83', 'shadow': '#73677d',
    'band_light': '#f2dbab', 'band_mid': '#d8b888', 'band_dark': '#ba9479',
    'limb': '#b6b6af', 'ring_back': '#baab9a', 'ring': '#d5c2a4',
    'ring_light': '#f0dcba', 'ring_dark': '#9d9691',
})
SKY = MappingProxyType({'top': '#344760', 'middle': '#202c42', 'bottom': '#34434d', 'star': '#c7d4d5'})
INTERIOR = MappingProxyType({
    'wall': '#c7a282', 'wall_light': '#d9b894', 'panel': '#487365',
    'panel_line': '#88a68c', 'worktop': '#597e70', 'edge': '#b2c2a1',
    'window_frame': '#ded0ab', 'window_light': '#f1dfb6',
    'structure': '#546c77', 'structure_light': '#a4b6aa',
})
WORK = MappingProxyType({
    'wall': '#59727a', 'wall_shadow': '#304c58', 'pipe': '#91aba8',
    'pipe_light': '#c5cbb5', 'pump': '#c79468', 'pump_light': '#e0b986',
    'screen': '#193442', 'warning': '#f0c783', 'warning_back': '#604f43',
    'safe': '#b2dfb9', 'safe_back': '#315e50', 'card_border': '#7bae97',
    'card_title': '#f4deb0', 'card_text': '#d6e2cd',
})
LORE = MappingProxyType({
    'background': '#101e1a', 'grid': '#20352a', 'centerline': '#40543e',
    'border': '#68845c', 'label': '#a5bc94', 'muted': '#91a780',
    'title': '#d1dbb6', 'portrait_background': '#486b5b',
})
FOLIO = MappingProxyType({'title': '#376657', 'muted': '#657267'})
REVIEW = MappingProxyType({
    'background': '#182c33', 'text': PAPER, 'mint': '#acd8b6',
    'paragraph': '#c2d1c6', 'surface': '#223b41', 'border': '#66857c',
    'focus': '#e1b581', 'button': '#243e43', 'button_text': '#172e32',
})

# The encyclopedia's existing channel treatment, applied to luminance.
LORE_CHANNELS = (
    ('feFuncR', MappingProxyType({'type': 'gamma', 'amplitude': 0.86, 'exponent': 1.25, 'offset': 0})),
    ('feFuncG', MappingProxyType({'type': 'linear', 'slope': 0.96, 'intercept': 0})),
    ('feFuncB', MappingProxyType({'type': 'gamma', 'amplitude': 0.81, 'exponent': 1.1, 'offset': 0})),
)
