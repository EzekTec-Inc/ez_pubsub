const pptxgen = require('pptxgenjs');

async function createPresentation() {
    let pptx = new pptxgen();
    
    // Layout and Theme
    pptx.layout = 'LAYOUT_16x9';
    
    // Colors
    const primary = '36454F'; // Charcoal
    const bg = 'F2F2F2'; // Off-white
    const accent = '212121'; // Black
    const codeBg = 'E8E8E8';
    
    pptx.defineSlideMaster({
        title: 'MASTER_SLIDE',
        background: { color: bg },
        objects: [
            { rect: { x: 0, y: 0, w: '100%', h: 0.2, fill: { color: primary } } }
        ]
    });

    // Slide 1: Title
    let slide1 = pptx.addSlide({ masterName: 'MASTER_SLIDE' });
    slide1.addText('ez_pubsub', {
        x: 0.5, y: 2.0, w: '90%', h: 1.0,
        fontSize: 60, bold: true, color: primary, fontFace: 'Arial Black'
    });
    slide1.addText('Lightweight, thread-safe publish-subscribe event bus for Rust', {
        x: 0.5, y: 3.2, w: '90%', h: 0.5,
        fontSize: 24, color: accent, fontFace: 'Arial'
    });

    // Slide 2: Executive Brief (NEW SLIDE)
    let slide2 = pptx.addSlide({ masterName: 'MASTER_SLIDE' });
    slide2.addText('Executive Brief', {
        x: 0.5, y: 0.5, w: '90%', h: 0.8,
        fontSize: 44, bold: true, color: primary, fontFace: 'Arial Black' // Larger title for brief
    });
    slide2.addText([
        { text: 'Simplified Integration', options: { bold: true, bullet: true } },
        { text: 'Decouples components for robust Rust applications.', options: { breakLine: true, indentLevel: 1, fontSize: 18 } },
        { text: 'Ideal for event-driven architectures, including terminal apps.', options: { breakLine: true, indentLevel: 1, fontSize: 18 } },
        
        { text: 'Key Benefits', options: { bold: true, bullet: true, margin: { top: 0.5 } } },
        { text: 'Thread-safe, type-safe events', options: { breakLine: true, indentLevel: 1, fontSize: 18 } },
        { text: 'Global macros for quick setup', options: { breakLine: true, indentLevel: 1, fontSize: 18 } },
        { text: 'Flexible subscription options (always/once)', options: { breakLine: true, indentLevel: 1, fontSize: 18 } },

        { text: 'Getting Started', options: { bold: true, bullet: true, margin: { top: 0.5 } } },
        { text: 'Add to Cargo.toml & run demo - fast integration.', options: { breakLine: true, indentLevel: 1, fontSize: 18 } },
    ], {
        x: 0.5, y: 1.5, w: '90%', h: 4.0, // Adjusted height
        fontSize: 20, color: accent, fontFace: 'Arial', bullet: true
    });


    // Slide 3: Features (Original Slide 2)
    let slide3 = pptx.addSlide({ masterName: 'MASTER_SLIDE' });
    slide3.addText('Why ez_pubsub?', {
        x: 0.5, y: 0.5, w: '90%', h: 0.8,
        fontSize: 36, bold: true, color: primary, fontFace: 'Arial Black'
    });
    slide3.addText([
        { text: 'Typed Events', options: { bold: true, bullet: true } },
        { text: 'Create a bus for any T: Send + Sync', options: { breakLine: true, indentLevel: 1 } },
        { text: 'Multiple Subscribers', options: { bold: true, bullet: true } },
        { text: 'Organize by target_id and callback_name', options: { breakLine: true, indentLevel: 1 } },
        { text: 'Flexible Execution', options: { bold: true, bullet: true } },
        { text: 'SubOption::Always or SubOption::Once', options: { breakLine: true, indentLevel: 1 } },
        { text: 'Global Macros', options: { bold: true, bullet: true } },
        { text: 'Zero-setup broadcast! and subscribe!', options: { breakLine: true, indentLevel: 1 } }
    ], {
        x: 0.5, y: 1.5, w: '90%', h: 3.0,
        fontSize: 20, color: accent, fontFace: 'Arial', bullet: true
    });

    // Slide 4: Instance API (Original Slide 3)
    let slide4 = pptx.addSlide({ masterName: 'MASTER_SLIDE' });
    slide4.addText('Instance API (Typed Bus)', {
        x: 0.5, y: 0.5, w: '90%', h: 0.8,
        fontSize: 36, bold: true, color: primary, fontFace: 'Arial Black'
    });
    
    const code1 = `let bus: PubSub<f32> = PubSub::new();\n\n// Always-on callback\nbus.subscribe("temperature", "display", "living_room", SubOption::Always, |temp| {\n    println!("Temp: {}°C", temp);\n});\n\n// Broadcast\nbus.broadcast("temperature", &22.5);`;
    
    slide4.addShape(pptx.ShapeType.rect, { x: 0.5, y: 1.5, w: 8.5, h: 2.8, fill: { color: codeBg } });
    slide4.addText(code1, {
        x: 0.6, y: 1.6, w: 8.3, h: 2.6,
        fontSize: 14, color: primary, fontFace: 'Consolas'
    });

    // Slide 5: Global Macros (Original Slide 4)
    let slide5 = pptx.addSlide({ masterName: 'MASTER_SLIDE' });
    slide5.addText('Global Bus Macros', {
        x: 0.5, y: 0.5, w: '90%', h: 0.8,
        fontSize: 36, bold: true, color: primary, fontFace: 'Arial Black'
    });

    const code2 = `// Short form — auto-named (useful for quick wiring)\nsubscribe!("motion", |loc: &String| {\n    println!("Anonymous handler: {}", loc);
});\n\nbroadcast!("motion", "front_door");\n\n// Remove all for target\nunsubscribe!("motion", "garage");`;

    slide5.addShape(pptx.ShapeType.rect, { x: 0.5, y: 1.5, w: 8.5, h: 2.8, fill: { color: codeBg } });
    slide5.addText(code2, {
        x: 0.6, y: 1.6, w: 8.3, h: 2.6,
        fontSize: 14, color: primary, fontFace: 'Consolas'
    });

    // Slide 6: Quick Start (Original Slide 5)
    let slide6 = pptx.addSlide({ masterName: 'MASTER_SLIDE' });
    slide6.addText('Getting Started', {
        x: 0.5, y: 0.5, w: '90%', h: 0.8,
        fontSize: 36, bold: true, color: primary, fontFace: 'Arial Black'
    });
    
    slide6.addText('1. Add to Cargo.toml:', {
        x: 0.5, y: 1.5, w: '90%', h: 0.4,
        fontSize: 20, bold: true, color: accent, fontFace: 'Arial'
    });
    slide6.addShape(pptx.ShapeType.rect, { x: 0.5, y: 2.0, w: 8.5, h: 0.8, fill: { color: codeBg } });
    slide6.addText('[dependencies]\nez_pubsub = "0.1"', {
        x: 0.6, y: 2.1, w: 8.3, h: 0.6,
        fontSize: 14, color: primary, fontFace: 'Consolas'
    });
    
    slide6.addText('2. Run the demo:', {
        x: 0.5, y: 3.2, w: '90%', h: 0.4,
        fontSize: 20, bold: true, color: accent, fontFace: 'Arial'
    });
    slide6.addShape(pptx.ShapeType.rect, { x: 0.5, y: 3.7, w: 8.5, h: 0.8, fill: { color: codeBg } });
    slide6.addText('cargo run --bin ez_pubsub_demo', {
        x: 0.6, y: 3.8, w: 8.3, h: 0.6,
        fontSize: 14, color: primary, fontFace: 'Consolas'
    });

    await pptx.writeFile({ fileName: 'ez_pubsub_intro.pptx' });
    console.log('Presentation created: ez_pubsub_intro.pptx');
}

createPresentation().catch(err => console.error(err));
