// The texture set's layout.json, for the version in textureSet.ts: where each
// part sits on the 400 x 524 frame canvas (px, [x0, y0, x1, y1]) and the two
// effects' lookup geometry. A copy of the numbers the set ships (the page's
// policy lets it fetch only from Supabase, so it cannot read the file from the
// host); test/unit/minionArt.test.ts checks the shared drawing accepts it. A
// new set version means a new copy.
// Rally's box is left out: it is not drawn.
export const TEXTURE_LAYOUT = {
  "canvas": {
    "width": 400,
    "height": 524
  },
  "frame": {
    "box": [0, 0, 400, 524]
  },
  "taunt": {
    "box": [-16, 0, 426, 592]
  },
  "parts": {
    "gem_attack": {
      "box": [12, 328, 146, 470]
    },
    "gem_health": {
      "box": [280, 325, 380, 474]
    },
    "skull": {
      "box": [120, 427, 284, 558]
    },
    "trigger": {
      "box": [159, 439, 261, 556]
    },
    "venomous": {
      "box": [168, -53, 247, 113]
    },
    "poisonous": {
      "box": [140, 412, 274, 545]
    },
    "divine_shield": {
      "box": [-94, -62, 503, 658]
    },
    "reborn_glow": {
      "box": [-154, -79, 543, 617]
    }
  },
  "windfury": {
    "box_canvas_px": [-36, 100, 440, 424],
    "size": [238, 162],
    "umin": -0.31,
    "umax": 1.8
  },
  "stealth": {
    "box_canvas_px": [24, 26, 378, 508],
    "size": [177, 241],
    "u": [0.0019193856312714479, -0.07833438025444973],
    "v": [-0.0019148090190243478, 1.2075484737226667]
  },
  "fx_canvas_pad_px": 100
};
