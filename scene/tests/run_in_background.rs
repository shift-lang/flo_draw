/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_scene::test::*;
use flo_scene::*;

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::prelude::*;

#[test]
fn say_hello_in_background() {
    let scene = Scene::empty();
    let hello_entity = EntityId::new();
    let (mut string_sender, mut string_receiver) = mpsc::channel(5);
    let (mut relay_sender, mut relay_receiver) = mpsc::channel(5);

    // Create an entity that monitors string_receiver in the background
    scene
        .create_entity(hello_entity, |_context, mut msg| async move {
            scene_run_in_background(async move {
                while let Some(string) = string_receiver.next().await {
                    relay_sender.send(string).await.ok();
                }
            })
            .unwrap();

            // Messages don't really matter here
            while let Some(msg) = msg.next().await {
                let _msg: String = msg;
            }
        })
        .unwrap();

    // Create a test for this scene
    scene
        .create_entity(TEST_ENTITY, move |_context, mut msg| async move {
            // Whenever a test is requested...
            while let Some(msg) = msg.next().await {
                let SceneTestRequest(mut msg) = msg;

                // Send a message to the background future
                string_sender.send("Hello".to_string()).await.ok();

                // Should receive another message from the receiver
                let received = relay_receiver.next().await;

                // Wait for the response, and succeed if the result is 'world'
                msg.send((received == Some("Hello".to_string())).into())
                    .await
                    .unwrap();
            }
        })
        .unwrap();

    // Test the scene we just set up
    test_scene(scene);
}

#[test]
fn say_hello_in_background_using_context() {
    let scene = Scene::empty();
    let hello_entity = EntityId::new();
    let (mut string_sender, mut string_receiver) = mpsc::channel(5);
    let (mut relay_sender, mut relay_receiver) = mpsc::channel(5);

    // Create an entity that monitors string_receiver in the background
    scene
        .create_entity(hello_entity, |context, mut msg| async move {
            context
                .run_in_background(async move {
                    while let Some(string) = string_receiver.next().await {
                        relay_sender.send(string).await.ok();
                    }
                })
                .unwrap();

            // Messages don't really matter here
            while let Some(msg) = msg.next().await {
                let _msg: String = msg;
            }
        })
        .unwrap();

    // Create a test for this scene
    scene
        .create_entity(TEST_ENTITY, move |_context, mut msg| async move {
            // Whenever a test is requested...
            while let Some(msg) = msg.next().await {
                let SceneTestRequest(mut msg) = msg;

                // Send a message to the background future
                string_sender.send("Hello".to_string()).await.ok();

                // Should receive another message from the receiver
                let received = relay_receiver.next().await;

                // Wait for the response, and succeed if the result is 'world'
                msg.send((received == Some("Hello".to_string())).into())
                    .await
                    .unwrap();
            }
        })
        .unwrap();

    // Test the scene we just set up
    test_scene(scene);
}

#[test]
fn say_hello_in_background_when_sealed() {
    let scene = Scene::empty();
    let hello_entity = EntityId::new();
    let (mut string_sender, mut string_receiver) = mpsc::channel(5);
    let (mut relay_sender, mut relay_receiver) = mpsc::channel(5);

    // Create an entity that monitors string_receiver in the background
    // As the entity is sealed, we need to hold on to a channel to stop it from being shut down
    let _hello_channel = scene
        .create_entity(hello_entity, move |context, mut msg| async move {
            context.seal_entity(hello_entity).unwrap();

            context
                .run_in_background(async move {
                    while let Some(string) = string_receiver.next().await {
                        println!("Run in background: {:?}", string);
                        relay_sender.send(string).await.ok();
                    }
                })
                .unwrap();

            // Messages don't really matter here
            while let Some(msg) = msg.next().await {
                let _msg: String = msg;
            }
        })
        .unwrap();

    // Create a test for this scene
    scene
        .create_entity(TEST_ENTITY, move |_context, mut msg| async move {
            // Whenever a test is requested...
            while let Some(msg) = msg.next().await {
                let SceneTestRequest(mut msg) = msg;

                // Send a message to the background future
                string_sender.send("Hello".to_string()).await.ok();

                // Should receive another message from the receiver
                let received = relay_receiver.next().await;
                println!("Received: {:?}", received);

                // Wait for the response, and succeed if the result is 'world'
                msg.send((received == Some("Hello".to_string())).into())
                    .await
                    .unwrap();
            }
        })
        .unwrap();

    // Test the scene we just set up
    test_scene(scene);
}

#[test]
fn background_has_current_scene() {
    let scene = Scene::empty();

    // Create a test for this scene
    scene
        .create_entity(TEST_ENTITY, move |context, mut msg| async move {
            // Whenever a test is requested...
            while let Some(msg) = msg.next().await {
                let SceneTestRequest(mut msg) = msg;

                let (sender, receiver) = oneshot::channel();
                context
                    .run_in_background(async move {
                        sender
                            .send(SceneContext::current().entity() == Some(TEST_ENTITY))
                            .ok();
                    })
                    .unwrap();

                let is_ok = receiver.await.unwrap();

                // Wait for the response, and succeed if the result is 'world'
                msg.send(is_ok.into()).await.unwrap();
            }
        })
        .unwrap();

    // Test the scene we just set up
    test_scene(scene);
}
