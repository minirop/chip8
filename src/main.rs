mod chip;

fn main() {
    let mut chip = chip::Chip::new();
    // chip.load("1-chip8-logo.ch8");
    // chip.load("2-ibm-logo.ch8");
    // chip.load("3-corax+.ch8");
    chip.load("4-flags.ch8");
    chip.reset();

    for _ in 0..2000 {
        chip.tick();
    }

    for (i, val) in chip.screen.iter().enumerate() {
        print!("{}", if *val { 'X' } else { ' ' });
        if (i % 64) == 63 {
            println!("");
        }
    }
}
