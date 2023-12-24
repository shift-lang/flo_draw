use flo_stream::*;
use futures::executor;
use futures::prelude::*;

use flo_draw::*;
use flo_render::*;

///
/// Simple example that displays a render window and renders a circle
///
pub fn main() {
    // 'with_2d_graphics' is used to support operating systems that can't run event loops anywhere other than the main thread
    with_2d_graphics(|| {
        // Create a render window and loop until it stops sending events
        executor::block_on(async {
            // Create a window
            let (mut renderer, mut events) = create_render_window("Direct render action window");

            // Render the instructions generaated by the show_tessellation example
            renderer
                .publish(vec![
                    RenderAction::CreateRenderTarget(
                        RenderTargetId(1),
                        TextureId(1),
                        Size2D(768, 768),
                        RenderTargetType::MonochromeMultisampledTexture,
                    ),
                    RenderAction::CreateRenderTarget(
                        RenderTargetId(0),
                        TextureId(0),
                        Size2D(768, 768),
                        RenderTargetType::Multisampled,
                    ),
                    RenderAction::SelectRenderTarget(RenderTargetId(0)),
                    RenderAction::BlendMode(BlendMode::SourceOver),
                    RenderAction::Clear(Rgba8([0, 0, 0, 0])),
                    RenderAction::SetTransform(Matrix([
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 1.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ])),
                    RenderAction::CreateVertex2DBuffer(
                        VertexBufferId(1),
                        vec![
                            Vertex2D {
                                pos: [499.99997, 250.0],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [516.01044, 250.50835],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [483.98947, 250.50836],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [468.10352, 252.03094],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [531.8965, 252.03098],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [452.3108, 254.57007],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [547.6891, 254.57007],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [436.58105, 258.1359],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [563.4189, 258.13593],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [421.14032, 262.69733],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [578.8597, 262.69736],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [406.0046, 268.27338],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [593.99536, 268.27338],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [391.1442, 274.88055],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [608.85583, 274.88055],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [376.53217, 282.5437],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [623.4679, 282.5437],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [362.51776, 291.1203],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [637.48224, 291.12033],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [348.97592, 300.73413],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [651.02405, 300.73413],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [335.88367, 311.42053],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [664.11633, 311.42053],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [323.2233, 323.2233],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [676.7767, 323.2233],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [311.42053, 335.88367],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [688.57947, 335.88367],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [300.73413, 348.97592],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [699.26587, 348.97595],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [291.1203, 362.51776],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [708.8797, 362.5178],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [282.5437, 376.53217],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [717.4563, 376.53217],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [725.1194, 391.14417],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [274.88055, 391.1442],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [268.27338, 406.0046],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [731.72656, 406.0046],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [262.69733, 421.14032],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [737.3026, 421.14032],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [258.1359, 436.58105],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [741.864, 436.58105],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [254.57007, 452.3108],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [745.43, 452.31085],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [252.03094, 468.10352],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [747.969, 468.10355],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [250.50836, 483.98947],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [749.4917, 483.9895],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [250.0, 499.99997],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [750.00006, 499.99997],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [250.50835, 516.01044],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [749.4917, 516.01044],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [252.03098, 531.8965],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [747.9692, 531.8965],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [254.57007, 547.6891],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [745.42993, 547.68915],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [258.13593, 563.4189],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [741.8641, 563.4189],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [262.69736, 578.8597],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [737.30273, 578.8597],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [268.27338, 593.99536],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [731.7267, 593.99536],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [274.88055, 608.85583],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [725.1195, 608.85583],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [282.5437, 623.4679],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [717.45636, 623.4679],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [291.12033, 637.48224],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [708.8797, 637.48224],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [300.73413, 651.02405],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [699.26587, 651.02405],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [311.42053, 664.11633],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [688.57947, 664.11633],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [323.2233, 676.7767],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [676.7767, 676.7767],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [335.88367, 688.57947],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [664.11633, 688.57947],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [348.97595, 699.26587],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [651.02405, 699.26587],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [362.5178, 708.8797],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [637.48224, 708.8797],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [376.53217, 717.4563],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [623.4679, 717.45636],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [391.14417, 725.1194],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [608.85583, 725.1195],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [406.0046, 731.72656],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [593.99536, 731.7267],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [421.14032, 737.3026],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [578.8597, 737.30273],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [436.58105, 741.864],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [563.4189, 741.8641],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [547.68915, 745.42993],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [452.31085, 745.43],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [468.10355, 747.969],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [531.8965, 747.9692],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [483.9895, 749.4917],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [516.01044, 749.4917],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                            Vertex2D {
                                pos: [499.99997, 750.00006],
                                tex_coord: [0.0, 0.0],
                                color: [76, 153, 204, 255],
                            },
                        ],
                    ),
                    RenderAction::CreateIndexBuffer(
                        IndexBufferId(1),
                        vec![
                            1, 0, 2, 2, 3, 5, 5, 7, 9, 9, 11, 13, 13, 15, 17, 17, 19, 21, 21, 23,
                            25, 25, 27, 29, 29, 31, 34, 2, 34, 35, 2, 5, 9, 9, 13, 17, 17, 21, 25,
                            25, 29, 34, 2, 9, 17, 17, 25, 34, 2, 17, 34, 1, 2, 35, 4, 1, 6, 8, 6,
                            10, 12, 10, 14, 16, 14, 18, 20, 18, 22, 24, 22, 26, 28, 26, 30, 32, 30,
                            33, 1, 36, 33, 6, 1, 10, 14, 10, 18, 22, 18, 26, 30, 26, 33, 10, 1, 18,
                            26, 18, 33, 18, 1, 33, 1, 35, 36, 35, 37, 39, 39, 41, 43, 43, 45, 47,
                            47, 49, 51, 51, 53, 55, 55, 57, 59, 59, 61, 63, 63, 65, 67, 67, 69, 71,
                            71, 73, 75, 75, 77, 79, 79, 81, 83, 83, 85, 87, 87, 90, 91, 35, 39, 43,
                            43, 47, 51, 51, 55, 59, 59, 63, 67, 67, 71, 75, 75, 79, 83, 83, 87, 91,
                            35, 43, 51, 51, 59, 67, 67, 75, 83, 35, 83, 91, 35, 51, 67, 35, 67, 83,
                            36, 35, 91, 38, 36, 40, 42, 40, 44, 46, 44, 48, 50, 48, 52, 54, 52, 56,
                            58, 56, 60, 62, 60, 64, 66, 64, 68, 70, 68, 72, 74, 72, 76, 78, 76, 80,
                            82, 80, 84, 86, 84, 88, 89, 88, 92, 40, 36, 44, 48, 44, 52, 56, 52, 60,
                            64, 60, 68, 72, 68, 76, 80, 76, 84, 88, 84, 92, 44, 36, 52, 60, 52, 68,
                            76, 68, 84, 36, 92, 84, 52, 36, 68, 36, 84, 68, 36, 91, 92, 92, 91, 93,
                            92, 93, 94, 94, 93, 95,
                        ],
                    ),
                    RenderAction::CreateVertex2DBuffer(
                        VertexBufferId(2),
                        vec![
                            Vertex2D {
                                pos: [686.31775, 662.1428],
                                tex_coord: [17.308632, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [690.84106, 666.08984],
                                tex_coord: [17.308632, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [696.878, 649.20514],
                                tex_coord: [34.208584, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [701.65375, 652.84296],
                                tex_coord: [34.208584, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [706.37427, 635.8289],
                                tex_coord: [50.81605, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [711.38513, 639.13544],
                                tex_coord: [50.81605, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [714.84564, 621.9865],
                                tex_coord: [67.24656, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [720.06696, 624.94904],
                                tex_coord: [67.24656, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [722.4176, 607.5483],
                                tex_coord: [83.74614, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [727.8213, 610.16315],
                                tex_coord: [83.74614, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [728.94543, 592.8661],
                                tex_coord: [100.00922, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [734.5077, 595.1244],
                                tex_coord: [100.00922, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [734.4535, 577.9151],
                                tex_coord: [116.13929, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [740.15173, 579.80414],
                                tex_coord: [116.13929, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [738.9596, 562.6615],
                                tex_coord: [132.23969, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [744.76843, 564.1763],
                                tex_coord: [132.23969, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [742.48303, 547.11884],
                                tex_coord: [148.36855, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [748.3767, 548.25946],
                                tex_coord: [148.36855, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [744.9919, 531.5149],
                                tex_coord: [164.36403, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [750.9462, 532.2781],
                                tex_coord: [164.36403, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [746.4963, 515.8196],
                                tex_coord: [180.32281, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [752.4871, 516.2014],
                                tex_coord: [180.32281, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [746.99854, 500.00003],
                                tex_coord: [196.34135, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [753.0016, 500.00003],
                                tex_coord: [196.34135, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [746.4963, 484.18045],
                                tex_coord: [212.35992, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [752.4871, 483.7986],
                                tex_coord: [212.35992, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [744.9918, 468.4852],
                                tex_coord: [228.3187, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [750.94617, 467.72195],
                                tex_coord: [228.3187, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [742.4831, 452.88116],
                                tex_coord: [244.31422, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [748.3768, 451.74054],
                                tex_coord: [244.31422, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [738.95966, 437.33847],
                                tex_coord: [260.4431, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [744.7685, 435.8237],
                                tex_coord: [260.4431, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [734.4536, 422.08487],
                                tex_coord: [276.5435, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [740.15186, 420.19583],
                                tex_coord: [276.5435, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [728.94556, 407.13385],
                                tex_coord: [292.67358, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [734.5078, 404.87555],
                                tex_coord: [292.67358, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [722.41766, 392.45166],
                                tex_coord: [308.93668, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [727.82135, 389.8368],
                                tex_coord: [308.93668, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [714.8457, 378.01346],
                                tex_coord: [325.43625, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [720.067, 375.05087],
                                tex_coord: [325.43625, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [706.3743, 364.17105],
                                tex_coord: [341.86673, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [711.3852, 360.86453],
                                tex_coord: [341.86673, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [696.87805, 350.79483],
                                tex_coord: [358.4742, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [701.6537, 347.15707],
                                tex_coord: [358.4742, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [686.3178, 337.85715],
                                tex_coord: [375.3741, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [690.8411, 333.9102],
                                tex_coord: [375.3741, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [674.65405, 325.34592],
                                tex_coord: [392.6828, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [678.8993, 321.10068],
                                tex_coord: [392.6828, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [662.1428, 313.6822],
                                tex_coord: [409.99146, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [666.08984, 309.15887],
                                tex_coord: [409.99146, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [649.20514, 303.12198],
                                tex_coord: [426.89136, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [652.84296, 298.34628],
                                tex_coord: [426.89136, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [635.8289, 293.62576],
                                tex_coord: [443.49884, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [639.13544, 288.61484],
                                tex_coord: [443.49884, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [621.9865, 285.1544],
                                tex_coord: [459.92935, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [624.94904, 279.933],
                                tex_coord: [459.92935, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [607.5483, 277.58243],
                                tex_coord: [476.42892, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [610.16315, 272.17874],
                                tex_coord: [476.42892, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [592.8661, 271.05447],
                                tex_coord: [492.69205, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [595.1244, 265.49222],
                                tex_coord: [492.69205, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [577.9151, 265.54648],
                                tex_coord: [508.8221, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [579.80414, 259.84818],
                                tex_coord: [508.8221, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [562.6615, 261.0403],
                                tex_coord: [524.9225, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [564.1763, 255.23148],
                                tex_coord: [524.9225, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [547.11884, 257.51688],
                                tex_coord: [541.05133, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [548.25946, 251.62321],
                                tex_coord: [541.05133, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [531.5149, 255.00812],
                                tex_coord: [557.0468, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [532.2781, 249.05377],
                                tex_coord: [557.0468, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [515.8196, 253.50381],
                                tex_coord: [573.0056, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [516.2014, 247.51291],
                                tex_coord: [573.0056, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [500.00003, 253.00151],
                                tex_coord: [589.0242, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [500.00003, 246.99849],
                                tex_coord: [589.0242, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [484.18042, 253.5038],
                                tex_coord: [605.0428, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [483.79858, 247.5129],
                                tex_coord: [605.0428, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [468.4852, 255.00815],
                                tex_coord: [621.0015, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [467.72195, 249.0538],
                                tex_coord: [621.0015, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [452.88116, 257.5169],
                                tex_coord: [636.9971, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [451.74054, 251.62323],
                                tex_coord: [636.9971, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [437.33847, 261.04034],
                                tex_coord: [653.126, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [435.8237, 255.23149],
                                tex_coord: [653.126, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [422.08487, 265.54648],
                                tex_coord: [669.2264, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [420.19583, 259.84818],
                                tex_coord: [669.2264, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [407.13382, 271.0545],
                                tex_coord: [685.3565, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [404.87552, 265.49225],
                                tex_coord: [685.3565, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [392.45166, 277.5824],
                                tex_coord: [701.61957, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [389.8368, 272.1787],
                                tex_coord: [701.61957, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [378.01346, 285.1544],
                                tex_coord: [718.11914, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [375.05087, 279.933],
                                tex_coord: [718.11914, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [364.17102, 293.62576],
                                tex_coord: [734.5496, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [360.86456, 288.6149],
                                tex_coord: [734.5496, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [350.79483, 303.122],
                                tex_coord: [751.15704, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [347.15707, 298.3463],
                                tex_coord: [751.15704, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [337.8571, 313.6822],
                                tex_coord: [768.057, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [333.91013, 309.15887],
                                tex_coord: [768.057, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [325.34592, 325.34592],
                                tex_coord: [785.36566, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [321.10068, 321.10068],
                                tex_coord: [785.36566, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [313.6822, 337.85715],
                                tex_coord: [802.6743, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [309.15887, 333.9102],
                                tex_coord: [802.6743, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [303.12198, 350.7948],
                                tex_coord: [819.5742, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [298.34628, 347.15704],
                                tex_coord: [819.5742, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [293.62573, 364.171],
                                tex_coord: [836.18164, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [288.61487, 360.86453],
                                tex_coord: [836.18164, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [285.1544, 378.01346],
                                tex_coord: [852.6121, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [279.933, 375.05087],
                                tex_coord: [852.6121, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [277.5824, 392.45163],
                                tex_coord: [869.1117, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [272.1787, 389.83676],
                                tex_coord: [869.1117, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [271.0545, 407.13376],
                                tex_coord: [885.37476, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [265.49225, 404.87546],
                                tex_coord: [885.37476, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [265.54648, 422.08484],
                                tex_coord: [901.50494, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [259.84818, 420.1958],
                                tex_coord: [901.50494, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [261.0403, 437.33844],
                                tex_coord: [917.60535, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [255.23146, 435.82367],
                                tex_coord: [917.60535, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [257.5169, 452.8811],
                                tex_coord: [933.7342, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [251.62323, 451.74048],
                                tex_coord: [933.7342, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [255.00812, 468.48514],
                                tex_coord: [949.72974, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [249.05377, 467.7219],
                                tex_coord: [949.72974, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [253.50381, 484.1804],
                                tex_coord: [965.6885, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [247.51291, 483.79855],
                                tex_coord: [965.6885, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [253.00151, 499.99997],
                                tex_coord: [981.70703, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [246.99849, 499.99997],
                                tex_coord: [981.70703, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [253.5038, 515.8195],
                                tex_coord: [997.7256, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [247.5129, 516.20135],
                                tex_coord: [997.7256, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [255.00815, 531.5149],
                                tex_coord: [1013.68445, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [249.0538, 532.2781],
                                tex_coord: [1013.68445, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [257.5169, 547.1188],
                                tex_coord: [1029.6798, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [251.62323, 548.2594],
                                tex_coord: [1029.6798, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [261.04034, 562.6615],
                                tex_coord: [1045.8087, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [255.23149, 564.1763],
                                tex_coord: [1045.8087, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [265.5465, 577.91516],
                                tex_coord: [1061.9092, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [259.8482, 579.8042],
                                tex_coord: [1061.9092, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [271.0545, 592.8662],
                                tex_coord: [1078.0393, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [265.49225, 595.1245],
                                tex_coord: [1078.0393, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [277.5824, 607.5484],
                                tex_coord: [1094.3024, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [272.1787, 610.16327],
                                tex_coord: [1094.3024, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [285.1544, 621.98663],
                                tex_coord: [1110.802, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [279.933, 624.94916],
                                tex_coord: [1110.802, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [293.62576, 635.829],
                                tex_coord: [1127.2324, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [288.6149, 639.1355],
                                tex_coord: [1127.2324, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [303.12198, 649.20514],
                                tex_coord: [1143.8398, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [298.34628, 652.84296],
                                tex_coord: [1143.8398, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [313.6822, 662.1428],
                                tex_coord: [1160.7397, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [309.15887, 666.08984],
                                tex_coord: [1160.7397, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [325.34592, 674.65405],
                                tex_coord: [1178.0483, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [321.10068, 678.8993],
                                tex_coord: [1178.0483, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [337.85715, 686.3178],
                                tex_coord: [1195.357, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [333.9102, 690.8411],
                                tex_coord: [1195.357, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [350.79483, 696.878],
                                tex_coord: [1212.257, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [347.15707, 701.65375],
                                tex_coord: [1212.257, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [364.17102, 706.37427],
                                tex_coord: [1228.8644, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [360.86456, 711.38513],
                                tex_coord: [1228.8644, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [378.01343, 714.84564],
                                tex_coord: [1245.2949, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [375.0509, 720.06696],
                                tex_coord: [1245.2949, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [392.4516, 722.41754],
                                tex_coord: [1261.7944, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [389.83673, 727.8212],
                                tex_coord: [1261.7944, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [407.13376, 728.94543],
                                tex_coord: [1278.0575, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [404.87546, 734.5077],
                                tex_coord: [1278.0575, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [422.08484, 734.4535],
                                tex_coord: [1294.1876, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [420.1958, 740.15173],
                                tex_coord: [1294.1876, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [437.33844, 738.9596],
                                tex_coord: [1310.288, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [435.82367, 744.76843],
                                tex_coord: [1310.288, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [452.88116, 742.48315],
                                tex_coord: [1326.4169, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [451.74054, 748.37683],
                                tex_coord: [1326.4169, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [468.48517, 744.9918],
                                tex_coord: [1342.4124, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [467.72192, 750.94617],
                                tex_coord: [1342.4124, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [484.18042, 746.4963],
                                tex_coord: [1358.3711, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [483.79858, 752.4871],
                                tex_coord: [1358.3711, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [499.99997, 746.99854],
                                tex_coord: [1374.3896, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [499.99997, 753.0016],
                                tex_coord: [1374.3896, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [515.8195, 746.4963],
                                tex_coord: [1390.4082, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [516.20135, 752.4871],
                                tex_coord: [1390.4082, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [531.5149, 744.992],
                                tex_coord: [1406.3671, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [532.2781, 750.94635],
                                tex_coord: [1406.3671, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [547.11884, 742.4831],
                                tex_coord: [1422.3625, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [548.25946, 748.3768],
                                tex_coord: [1422.3625, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [562.6615, 738.95966],
                                tex_coord: [1438.4915, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [564.1763, 744.7685],
                                tex_coord: [1438.4915, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [577.91516, 734.4536],
                                tex_coord: [1454.5919, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [579.8042, 740.15186],
                                tex_coord: [1454.5919, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [592.8662, 728.94556],
                                tex_coord: [1470.722, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [595.1245, 734.5078],
                                tex_coord: [1470.722, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [607.5484, 722.41766],
                                tex_coord: [1486.9851, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [610.16327, 727.82135],
                                tex_coord: [1486.9851, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [621.98663, 714.8457],
                                tex_coord: [1503.4847, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [624.94916, 720.067],
                                tex_coord: [1503.4847, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [635.829, 706.37427],
                                tex_coord: [1519.9153, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [639.1355, 711.38513],
                                tex_coord: [1519.9153, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [649.20514, 696.878],
                                tex_coord: [1536.5227, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [652.84296, 701.65375],
                                tex_coord: [1536.5227, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [662.1428, 686.3178],
                                tex_coord: [1553.4226, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [666.08984, 690.8411],
                                tex_coord: [1553.4226, 0.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [674.65405, 674.65405],
                                tex_coord: [1570.7313, 1.0],
                                color: [0, 0, 0, 255],
                            },
                            Vertex2D {
                                pos: [678.8993, 678.8993],
                                tex_coord: [1570.7313, 0.0],
                                color: [0, 0, 0, 255],
                            },
                        ],
                    ),
                    RenderAction::CreateIndexBuffer(
                        IndexBufferId(2),
                        vec![
                            0, 1, 2, 1, 3, 2, 2, 3, 4, 3, 5, 4, 4, 5, 6, 5, 7, 6, 6, 7, 8, 7, 9, 8,
                            8, 9, 10, 9, 11, 10, 10, 11, 12, 11, 13, 12, 12, 13, 14, 13, 15, 14,
                            14, 15, 16, 15, 17, 16, 16, 17, 18, 17, 19, 18, 18, 19, 20, 19, 21, 20,
                            20, 21, 22, 21, 23, 22, 22, 23, 24, 23, 25, 24, 24, 25, 26, 25, 27, 26,
                            26, 27, 28, 27, 29, 28, 28, 29, 30, 29, 31, 30, 30, 31, 32, 31, 33, 32,
                            32, 33, 34, 33, 35, 34, 34, 35, 36, 35, 37, 36, 36, 37, 38, 37, 39, 38,
                            38, 39, 40, 39, 41, 40, 40, 41, 42, 41, 43, 42, 42, 43, 44, 43, 45, 44,
                            44, 45, 46, 45, 47, 46, 46, 47, 48, 47, 49, 48, 48, 49, 50, 49, 51, 50,
                            50, 51, 52, 51, 53, 52, 52, 53, 54, 53, 55, 54, 54, 55, 56, 55, 57, 56,
                            56, 57, 58, 57, 59, 58, 58, 59, 60, 59, 61, 60, 60, 61, 62, 61, 63, 62,
                            62, 63, 64, 63, 65, 64, 64, 65, 66, 65, 67, 66, 66, 67, 68, 67, 69, 68,
                            68, 69, 70, 69, 71, 70, 70, 71, 72, 71, 73, 72, 72, 73, 74, 73, 75, 74,
                            74, 75, 76, 75, 77, 76, 76, 77, 78, 77, 79, 78, 78, 79, 80, 79, 81, 80,
                            80, 81, 82, 81, 83, 82, 82, 83, 84, 83, 85, 84, 84, 85, 86, 85, 87, 86,
                            86, 87, 88, 87, 89, 88, 88, 89, 90, 89, 91, 90, 90, 91, 92, 91, 93, 92,
                            92, 93, 94, 93, 95, 94, 94, 95, 96, 95, 97, 96, 96, 97, 98, 97, 99, 98,
                            98, 99, 100, 99, 101, 100, 100, 101, 102, 101, 103, 102, 102, 103, 104,
                            103, 105, 104, 104, 105, 106, 105, 107, 106, 106, 107, 108, 107, 109,
                            108, 108, 109, 110, 109, 111, 110, 110, 111, 112, 111, 113, 112, 112,
                            113, 114, 113, 115, 114, 114, 115, 116, 115, 117, 116, 116, 117, 118,
                            117, 119, 118, 118, 119, 120, 119, 121, 120, 120, 121, 122, 121, 123,
                            122, 122, 123, 124, 123, 125, 124, 124, 125, 126, 125, 127, 126, 126,
                            127, 128, 127, 129, 128, 128, 129, 130, 129, 131, 130, 130, 131, 132,
                            131, 133, 132, 132, 133, 134, 133, 135, 134, 134, 135, 136, 135, 137,
                            136, 136, 137, 138, 137, 139, 138, 138, 139, 140, 139, 141, 140, 140,
                            141, 142, 141, 143, 142, 142, 143, 144, 143, 145, 144, 144, 145, 146,
                            145, 147, 146, 146, 147, 148, 147, 149, 148, 148, 149, 150, 149, 151,
                            150, 150, 151, 152, 151, 153, 152, 152, 153, 154, 153, 155, 154, 154,
                            155, 156, 155, 157, 156, 156, 157, 158, 157, 159, 158, 158, 159, 160,
                            159, 161, 160, 160, 161, 162, 161, 163, 162, 162, 163, 164, 163, 165,
                            164, 164, 165, 166, 165, 167, 166, 166, 167, 168, 167, 169, 168, 168,
                            169, 170, 169, 171, 170, 170, 171, 172, 171, 173, 172, 172, 173, 174,
                            173, 175, 174, 174, 175, 176, 175, 177, 176, 176, 177, 178, 177, 179,
                            178, 178, 179, 180, 179, 181, 180, 180, 181, 182, 181, 183, 182, 182,
                            183, 184, 183, 185, 184, 184, 185, 186, 185, 187, 186, 186, 187, 188,
                            187, 189, 188, 188, 189, 190, 189, 191, 190, 190, 191, 0, 191, 1, 0,
                        ],
                    ),
                    RenderAction::SelectRenderTarget(RenderTargetId(0)),
                    RenderAction::BlendMode(BlendMode::SourceOver),
                    RenderAction::UseShader(ShaderType::Simple { clip_texture: None }),
                    RenderAction::SetTransform(Matrix([
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 1.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ])),
                    RenderAction::SetTransform(Matrix([
                        [0.002, 0.0, 0.0, -1.0],
                        [0.0, 0.002, 0.0, -1.0],
                        [0.0, 0.0, 1.0, 1.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ])),
                    RenderAction::DrawIndexedTriangles(VertexBufferId(1), IndexBufferId(1), 282),
                    RenderAction::DrawIndexedTriangles(VertexBufferId(2), IndexBufferId(2), 576),
                    RenderAction::RenderToFrameBuffer,
                    RenderAction::BlendMode(BlendMode::SourceOver),
                    RenderAction::SetTransform(Matrix([
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ])),
                    RenderAction::Clear(Rgba8([255, 255, 255, 255])),
                    RenderAction::DrawFrameBuffer(
                        RenderTargetId(0),
                        FrameBufferRegion::default(),
                        Alpha(1.0),
                    ),
                    RenderAction::ShowFrameBuffer,
                    RenderAction::FreeRenderTarget(RenderTargetId(0)),
                    RenderAction::FreeRenderTarget(RenderTargetId(1)),
                    RenderAction::FreeTexture(TextureId(0)),
                    RenderAction::FreeTexture(TextureId(1)),
                ])
                .await;

            // Wait until it stops producing events
            while let Some(evt) = events.next().await {
                // Stop reading events when the window is closed (this will close our streams, so the window will disappear)
                match evt {
                    DrawEvent::Closed => {
                        break;
                    }
                    _ => {}
                }
            }
        });
    });
}
