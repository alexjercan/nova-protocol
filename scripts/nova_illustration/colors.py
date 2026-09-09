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
SAMIR = MappingProxyType({
    'skin': '#ab8267', 'shadow': '#644c3e', 'light': '#d5b28c',
    'hair': '#35322d', 'hair_line': '#776957', 'brow': '#48382c',
    'eye': '#c5b59a', 'iris': '#323b32', 'glasses': '#444e49', 'lines': '#70503d',
    'coat': '#596961', 'coat_shadow': '#455b50', 'coat_light': '#9da995',
    'shirt': '#a5ad98',
})
NADIA = MappingProxyType({
    'skin': '#aa7f65', 'shadow': '#705045', 'light': '#dbb698',
    'hair': '#302f37', 'hair_line': '#79717b', 'brow': '#493830',
    'eye': '#e4cfb2', 'iris': '#483a34', 'lines': '#735447',
    'coat': '#687383', 'coat_shadow': '#434e60', 'coat_light': '#a4b0b9',
    'shirt': '#c4b7a0', 'trim': '#c9a669',
})
OWEN = MappingProxyType({
    'skin': '#c5a282', 'shadow': '#846558', 'light': '#e7c9a5',
    'hair': '#4c4943', 'hair_line': '#a49a85', 'brow': '#625043',
    'eye': '#ded0b5', 'iris': '#4b5349', 'lines': '#866550',
    'coat': '#607c7c', 'coat_shadow': '#405659', 'coat_light': '#a3bcbc',
    'shirt': '#cbc3ad', 'trim': '#b9a173',
})
IVO = MappingProxyType({
    'skin': '#c09a86', 'shadow': '#805951', 'light': '#e4c5ae',
    'hair': '#60483b', 'hair_line': '#a08369', 'brow': '#624636',
    'eye': '#e5d2b7', 'iris': '#536660', 'lines': '#855f50',
    'coat': '#9f6c54', 'coat_shadow': '#704b40', 'coat_light': '#d0a484',
    'shirt': '#b4b6a8', 'trim': '#c2ae83',
})
MATERIALS = MappingProxyType({
    'paint': '#b4b9b5', 'metal': '#84979b', 'jade': '#5c8e79',
    'tank': '#c7c7b5', 'shadow': '#52616c', 'dark': '#29313b',
    'plate': '#a7afb4', 'plate_light': '#cbd0c9', 'steel': '#676c70',
    'hazard': '#e0b352', 'scorch': '#4c3934',
    'assembly': '#c79468', 'assembly_face': '#e0b986',
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
FREIGHT = MappingProxyType({
    'wall': '#d2c6b2', 'wall_light': '#e5dcca', 'panel': '#7b959d',
    'panel_shadow': '#496572', 'frame': '#344d60', 'rail': '#b7c7bd',
    'edge': '#f0d8a8', 'stripe': '#bd955e', 'label': '#d7e5de',
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
