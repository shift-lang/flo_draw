use std::collections::HashSet;
use std::sync::*;
#[cfg(any(feature = "timer", feature = "test-scene"))]
use std::time::Duration;

use futures::future::BoxFuture;
use futures::prelude::*;
#[cfg(any(feature = "timer", feature = "test-scene"))]
use futures_timer::Delay;

use crate::context::*;
use crate::entity_channel::*;
use crate::entity_id::*;
use crate::error::*;
use crate::expected_entity_channel::*;
use crate::simple_entity_channel::*;

use super::entity_channel_ext::*;

///
/// A recipe is used to describe a set of actions sent to one or more entities in a scene, in order.
///
/// This is essentially a simple scripting extension, making it possible to encode fixed sets of steps into
/// a script that can be repeatedly executed (for more complicated scripting, a scripting language should
/// probably be used)
///
/// A recipe is useful in a number of situations, but in particular for testing where it can be used to describe a
/// set of messages and expected responses.
///
/// Create a simple recipe and run it:
///
/// ```
/// # use flo_scene::*;
/// # use futures::executor;
/// # use std::thread;
/// # let scene = Scene::default();
/// # let scene_context = scene.context();
/// # thread::spawn(move || executor::block_on(scene.run()));
/// let recipe = Recipe::new()
///     .send_generated_messages(EXAMPLE, || vec![ExampleRequest::Example]);
///
/// executor::block_on(async { recipe.run(&scene_context).await.unwrap(); });
/// ```
///
/// A more complicated recipe with an expected response:
///
/// ```
/// # use flo_scene::*;
/// # use futures::executor;
/// # use std::thread;
/// # let scene = Scene::default();
/// # let scene_context = scene.context();
/// # thread::spawn(move || executor::block_on(scene.run()));
/// let recipe = Recipe::new()
///     .expect(vec![Heartbeat])
///     .after_sending_messages(HEARTBEAT, |heartbeat_channel| vec![HeartbeatRequest::RequestHeartbeat(heartbeat_channel)])
///     .alongside_generated_messages(EXAMPLE, || vec![ExampleRequest::Example]);
///
/// executor::block_on(async { recipe.run(&scene_context).await.unwrap(); });
/// ```
///
#[derive(Clone)]
pub struct Recipe {
    /// The entity ID used for channels generated by this recipe
    entity_id: EntityId,

    /// Each step is a boxed function returning a future
    steps: Vec<Arc<dyn Send + Sync + Fn(Arc<SceneContext>) -> BoxFuture<'static, Result<(), RecipeError>>>>,
}

///
/// An intermediate build stage for a recipe step that expects a particular response to be sent to a channel
///
pub struct ExpectingRecipe<TExpectedChannel> {
    /// The recipe that the 'expect' step will be appended to 
    recipe: Recipe,

    /// Factory method to generate the expected response channel and a future for when the channel has generated all of its expected responses
    responses: Box<dyn Send + Sync + Fn(Arc<SceneContext>) -> (TExpectedChannel, BoxFuture<'static, Result<(), RecipeError>>)>,
}

impl Default for Recipe {
    ///
    /// Creates a default (empty) recipe
    ///
    fn default() -> Recipe {
        Recipe {
            entity_id: EntityId::new(),
            steps: vec![],
        }
    }
}

impl Recipe {
    ///
    /// Creates a new empty recipe
    ///
    pub fn new() -> Recipe {
        Self::default()
    }

    ///
    /// Runs this recipe
    ///
    pub async fn run(&self, context: &Arc<SceneContext>) -> Result<(), RecipeError> {
        // Run the steps in the recipe, stop if any of them generate an error
        for step in self.steps.iter() {
            step(Arc::clone(context)).await?;
        }

        Ok(())
    }

    ///
    /// Runs this recipe with a timeout
    ///
    /// Requires the `timer` feature.
    ///
    #[cfg(any(feature = "timer", feature = "test-scene"))]
    pub async fn run_with_timeout(&self, context: &Arc<SceneContext>, timeout: Duration) -> Result<(), RecipeError> {
        // The timeout future is used to abort the test if it takes too long
        let timeout = Delay::new(timeout)
            .map(|_| Err(RecipeError::Timeout));

        // Create a future to run the steps
        let steps = self.steps.clone();
        let run_steps = async move {
            for step in steps.into_iter() {
                step(Arc::clone(context)).await?;
            }

            Ok(())
        };

        // Pick whichever of the two futures finishes first
        let run_steps = run_steps.boxed_local();
        let timeout = timeout.boxed_local();
        let result = future::select_all(vec![run_steps, timeout]);
        let (result, _, _) = result.await;

        result
    }

    ///
    /// Creates a step for sending a generated set of messages
    ///
    fn create_generated_step<TMessageIterator>(target_entity_id: EntityId, generate_messages: impl 'static + Send + Sync + Fn() -> TMessageIterator) -> Arc<dyn Send + Sync + Fn(Arc<SceneContext>) -> BoxFuture<'static, Result<(), RecipeError>>>
        where
            TMessageIterator: 'static + IntoIterator,
            TMessageIterator::IntoIter: 'static + Send,
            TMessageIterator::Item: 'static + Send,
    {
        Arc::new(move |context: Arc<SceneContext>| {
            let messages = generate_messages().into_iter();

            async move {
                // Send to the entity
                let mut channel = context.send_to(target_entity_id)?;

                // Copy the messages one at a time
                for msg in messages {
                    channel.send(msg).await?;
                }

                Ok(())
            }.boxed()
        })
    }

    ///
    /// Adds a new step to the recipe that sends a set of fixed messages to an entity
    ///
    pub fn send_messages<TMessage>(self, target_entity_id: EntityId, messages: impl IntoIterator<Item=TMessage>) -> Recipe
        where
            TMessage: 'static + Clone + Send,
    {
        let messages = Mutex::new(messages.into_iter().collect::<Vec<_>>());
        self.send_generated_messages(target_entity_id, move || messages.lock().unwrap().clone())
    }

    ///
    /// Adds a new step to the recipe that sends a set of messages generated by a function to an entity
    ///
    /// This can be used for sending messages that are not `Clone`. For messages that send responses to a channel, see `expect()`
    ///
    pub fn send_generated_messages<TMessageIterator>(self, target_entity_id: EntityId, generate_messages: impl 'static + Send + Sync + Fn() -> TMessageIterator) -> Recipe
        where
            TMessageIterator: 'static + IntoIterator,
            TMessageIterator::IntoIter: 'static + Send,
            TMessageIterator::Item: 'static + Send,
    {
        let our_entity_id = self.entity_id;
        let mut steps = self.steps;
        let new_step = Self::create_generated_step(target_entity_id, generate_messages);

        steps.push(new_step);

        Recipe {
            entity_id: our_entity_id,
            steps: steps,
        }
    }

    ///
    /// Amends the step on top of the recipe so that it sends the a set of messages in parallel with the existing step
    ///
    pub fn alongside_messages<TMessage>(self, target_entity_id: EntityId, messages: impl IntoIterator<Item=TMessage>) -> Recipe
        where
            TMessage: 'static + Clone + Send,
    {
        let messages = Mutex::new(messages.into_iter().collect::<Vec<_>>());
        self.alongside_generated_messages(target_entity_id, move || messages.lock().unwrap().clone())
    }

    ///
    /// Amends the step on top of the recipe so that it sends the a set of generated messages in parallel with the existing step
    ///
    pub fn alongside_generated_messages<TMessageIterator>(self, target_entity_id: EntityId, generate_messages: impl 'static + Send + Sync + Fn() -> TMessageIterator) -> Recipe
        where
            TMessageIterator: 'static + IntoIterator,
            TMessageIterator::IntoIter: 'static + Send,
            TMessageIterator::Item: 'static + Send,
    {
        let our_entity_id = self.entity_id;
        let mut steps = self.steps;
        let new_step = Self::create_generated_step(target_entity_id, generate_messages);

        if let Some(old_step) = steps.pop() {
            // Combine the new step with the old_step
            let combined_step = Arc::new(move |context: Arc<SceneContext>| {
                let old_step = old_step(Arc::clone(&context));
                let new_step = new_step(context);

                // Fold the errors into a single error
                async move {
                    let results = future::join_all(vec![old_step, new_step]).await;
                    results.into_iter()
                        .fold(Ok(()), fold_recipe_error)
                }.boxed()
            });

            // Replace the old step with the combined step
            steps.push(combined_step)
        } else {
            // In the event this is called with no steps, just send the messages normally
            steps.push(new_step);
        }

        Recipe {
            entity_id: our_entity_id,
            steps: steps,
        }
    }

    ///
    /// Starts to define a step that expects a specific set of responses to be sent to channel
    ///
    /// A channel that will process the responses is supplied to a factory method
    ///
    pub fn expect<TResponse>(self, responses: impl IntoIterator<Item=TResponse>) -> ExpectingRecipe<BoxedEntityChannel<'static, TResponse>>
        where
            TResponse: 'static + Send + Sync + PartialEq,
    {
        let entity_id = self.entity_id;
        let responses = responses.into_iter().collect::<Vec<_>>();
        let responses = Arc::new(responses);

        ExpectingRecipe {
            recipe: self,
            responses: Box::new(move |_context| {
                let (channel, future) = ExpectedEntityChannel::new(entity_id, Arc::clone(&responses));

                (channel.boxed(), future.boxed())
            }),
        }
    }

    ///
    /// Creates the 'expecting' function for matching a channel against a set of responses that we need to wait for
    ///
    fn wait_for_channel<TResponse>(our_entity_id: EntityId, responses: Vec<TResponse>) -> impl Send + Sync + Fn(Arc<SceneContext>) -> (BoxedEntityChannel<'static, TResponse>, BoxFuture<'static, Result<(), RecipeError>>)
        where
            TResponse: 'static + Send + Sync + PartialEq,
    {
        let responses = Arc::new(responses);

        move |_| {
            // Take our own copy of the responses
            let responses = Arc::clone(&responses);

            // Create a simple entity channel that we'll receive the messages from
            let (channel, receiver) = SimpleEntityChannel::new(our_entity_id, 1);

            // The future reads from the channel until all of the responses are received
            let future = async move {
                // Match immediately if there's no sequence to wait for
                if responses.len() == 0 {
                    return Ok(());
                }

                // Start waiting for the sequence at position 0
                let mut pos = 0;
                let mut receiver = receiver;

                while let Some(msg) = receiver.next().await {
                    if &msg == &responses[pos] {
                        // Matched the next expected response
                        pos += 1;

                        if pos >= responses.len() {
                            // Matched the whole set of responses
                            return Ok(());
                        }
                    }
                }

                // Channel closed before we received all of the responses
                Err(RecipeError::ExpectedMoreResponses)
            };

            (channel.boxed(), future.boxed())
        }
    }

    ///
    /// Creates the 'expecting' function for matching a channel against a set of responses that we need to wait for (matching them in any order)
    ///
    fn wait_for_unordered_channel<TResponse>(our_entity_id: EntityId, responses: Vec<TResponse>) -> impl Send + Sync + Fn(Arc<SceneContext>) -> (BoxedEntityChannel<'static, TResponse>, BoxFuture<'static, Result<(), RecipeError>>)
        where
            TResponse: 'static + Send + Sync + PartialEq,
    {
        let responses = Arc::new(responses);

        move |_| {
            // Take our own copy of the responses
            let responses = Arc::clone(&responses);

            // Create a simple entity channel that we'll receive the messages from
            let (channel, receiver) = SimpleEntityChannel::new(our_entity_id, 1);

            // The future reads from the channel until all of the responses are received
            let future = async move {
                // Match immediately if there's no sequence to wait for
                if responses.len() == 0 {
                    return Ok(());
                }

                // Start waiting for the sequence at position 0
                let mut matches = HashSet::new();
                let mut receiver = receiver;

                while let Some(msg) = receiver.next().await {
                    for pos in 0..responses.len() {
                        if !matches.contains(&pos) && &msg == &responses[pos] {
                            // Matched a response we haven't seen before
                            matches.insert(pos);

                            if matches.len() >= responses.len() {
                                // Matched the whole set of responses
                                return Ok(());
                            }

                            break;
                        }
                    }
                }

                // Channel closed before we received all of the responses
                Err(RecipeError::ExpectedMoreResponses)
            };

            (channel.boxed(), future.boxed())
        }
    }

    ///
    /// Similar to `expect()`, this will wait for the supplied set of responses to be returned in order, but will ignore responses that
    /// don't match what was expected rather than erroring out (ie, will wait until the full response sequence is seen)
    ///
    pub fn wait_for<TResponse>(self, responses: impl IntoIterator<Item=TResponse>) -> ExpectingRecipe<BoxedEntityChannel<'static, TResponse>>
        where
            TResponse: 'static + Send + Sync + PartialEq,
    {
        let entity_id = self.entity_id;

        ExpectingRecipe {
            recipe: self,
            responses: Box::new(Self::wait_for_channel(entity_id, responses.into_iter().collect())),
        }
    }

    ///
    /// Similar to `expect()`, this will wait for the supplied set of responses to be returned, in any order, before passing.
    ///
    /// Supplying a response more than once will cause it to be matched for that many times
    ///
    pub fn wait_for_unordered<TResponse>(self, responses: impl IntoIterator<Item=TResponse>) -> ExpectingRecipe<BoxedEntityChannel<'static, TResponse>>
        where
            TResponse: 'static + Send + Sync + PartialEq,
    {
        let entity_id = self.entity_id;

        ExpectingRecipe {
            recipe: self,
            responses: Box::new(Self::wait_for_unordered_channel(entity_id, responses.into_iter().collect())),
        }
    }

    // TODO: add an '.alongside_expect()' function for expecting on another channel in parallel with another entity
    // TODO: some way to describe which part of the recipe failed in the error
    // TODO: a way of waiting for expected messages asynchronously with the following steps in the recipe
}

///
/// Combines two reuslts into a single error
///
fn fold_recipe_error(a: Result<(), RecipeError>, b: Result<(), RecipeError>) -> Result<(), RecipeError> {
    match (a, b) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), Ok(())) => Err(err),
        (Ok(()), Err(err)) => Err(err),

        (Err(RecipeError::ManyErrors(mut a)), Err(RecipeError::ManyErrors(b))) => {
            a.extend(b);
            Err(RecipeError::ManyErrors(a))
        }

        (Err(RecipeError::ManyErrors(mut a)), Err(b)) => {
            a.push(b);
            Err(RecipeError::ManyErrors(a))
        }

        (Err(a), Err(RecipeError::ManyErrors(mut b))) => {
            b.insert(0, a);
            Err(RecipeError::ManyErrors(b))
        }

        (Err(a), Err(b)) => {
            Err(RecipeError::ManyErrors(vec![a, b]))
        }
    }
}

impl<TExpectedChannel> ExpectingRecipe<TExpectedChannel>
    where
        TExpectedChannel: 'static + Send,
{
    ///
    /// Sends the messages that expect this response
    ///
    pub fn after_sending_messages<TMessageIterator>(self, target_entity_id: EntityId, generate_messages: impl 'static + Send + Sync + Fn(TExpectedChannel) -> TMessageIterator) -> Recipe
        where
            TMessageIterator: 'static + IntoIterator,
            TMessageIterator::IntoIter: 'static + Send,
            TMessageIterator::Item: 'static + Send,
    {
        let our_entity_id = self.recipe.entity_id;
        let mut steps = self.recipe.steps;
        let responses = self.responses;

        let new_step = Arc::new(move |context: Arc<SceneContext>| {
            let (channel, future) = responses(Arc::clone(&context));
            let messages = generate_messages(channel).into_iter().collect::<Vec<_>>();

            async move {
                // Send to the entity
                let mut channel = context.send_to(target_entity_id)?;

                // Copy the messages one at a time
                for msg in messages {
                    channel.send(msg).await?;
                }

                // Wait for the expected responses to arrive
                future.await?;

                Ok(())
            }.boxed()
        });

        steps.push(new_step);

        Recipe {
            entity_id: our_entity_id,
            steps: steps,
        }
    }
}

impl<TResponse1> ExpectingRecipe<BoxedEntityChannel<'static, TResponse1>>
    where
        TResponse1: 'static + Send + Sync + PartialEq,
{
    ///
    /// As for `Recipe::expect`, except this will extend the number of channels with expectations to 2 
    ///
    pub fn expect<TResponse2>(self, responses: impl IntoIterator<Item=TResponse2>) -> ExpectingRecipe<(BoxedEntityChannel<'static, TResponse1>, BoxedEntityChannel<'static, TResponse2>)>
        where
            TResponse2: 'static + Send + Sync + PartialEq,
    {
        let recipe = self.recipe;
        let entity_id = recipe.entity_id;
        let other_responses = self.responses;
        let responses = responses.into_iter().collect::<Vec<_>>();
        let responses = Arc::new(responses);

        ExpectingRecipe {
            recipe: recipe,
            responses: Box::new(move |context| {
                // Request the other channel
                let (other_channel, other_future) = other_responses(context);

                // Create the this channel
                let (our_channel, our_future) = ExpectedEntityChannel::new(entity_id, Arc::clone(&responses));

                let future = async move {
                    let all_responses = future::join_all(vec![other_future, our_future.boxed()]).await;
                    all_responses.into_iter()
                        .fold(Ok(()), fold_recipe_error)
                };

                ((other_channel, our_channel.boxed()), future.boxed())
            }),
        }
    }
}

impl<TResponse1, TResponse2> ExpectingRecipe<(BoxedEntityChannel<'static, TResponse1>, BoxedEntityChannel<'static, TResponse2>)>
    where
        TResponse1: 'static + Send + Sync + PartialEq,
        TResponse2: 'static + Send + Sync + PartialEq,
{
    ///
    /// As for `Recipe::expect`, except this will extend the number of channels with expectations to 2 
    ///
    pub fn expect<TResponse3>(self, responses: impl IntoIterator<Item=TResponse3>) -> ExpectingRecipe<(BoxedEntityChannel<'static, TResponse1>, BoxedEntityChannel<'static, TResponse2>, BoxedEntityChannel<'static, TResponse3>)>
        where
            TResponse3: 'static + Send + Sync + PartialEq,
    {
        let recipe = self.recipe;
        let entity_id = recipe.entity_id;
        let other_responses = self.responses;
        let responses = responses.into_iter().collect::<Vec<_>>();
        let responses = Arc::new(responses);

        ExpectingRecipe {
            recipe: recipe,
            responses: Box::new(move |context| {
                // Request the other channel
                let ((other_channel1, other_channel2), other_future) = other_responses(context);

                // Create the this channel
                let (our_channel, our_future) = ExpectedEntityChannel::new(entity_id, Arc::clone(&responses));

                let future = async move {
                    let all_responses = future::join_all(vec![other_future, our_future.boxed()]).await;
                    all_responses.into_iter()
                        .fold(Ok(()), fold_recipe_error)
                };

                ((other_channel1, other_channel2, our_channel.boxed()), future.boxed())
            }),
        }
    }
}

impl<TResponse1, TResponse2, TResponse3> ExpectingRecipe<(BoxedEntityChannel<'static, TResponse1>, BoxedEntityChannel<'static, TResponse2>, BoxedEntityChannel<'static, TResponse3>)>
    where
        TResponse1: 'static + Send + Sync + PartialEq,
        TResponse2: 'static + Send + Sync + PartialEq,
        TResponse3: 'static + Send + Sync + PartialEq,
{
    ///
    /// As for `Recipe::expect`, except this will extend the number of channels with expectations to 2 
    ///
    pub fn expect<TResponse4>(self, responses: impl IntoIterator<Item=TResponse4>) -> ExpectingRecipe<(BoxedEntityChannel<'static, TResponse1>, BoxedEntityChannel<'static, TResponse2>, BoxedEntityChannel<'static, TResponse3>, BoxedEntityChannel<'static, TResponse4>)>
        where
            TResponse4: 'static + Send + Sync + PartialEq,
    {
        let recipe = self.recipe;
        let entity_id = recipe.entity_id;
        let other_responses = self.responses;
        let responses = responses.into_iter().collect::<Vec<_>>();
        let responses = Arc::new(responses);

        ExpectingRecipe {
            recipe: recipe,
            responses: Box::new(move |context| {
                // Request the other channel
                let ((other_channel1, other_channel2, other_channel3), other_future) = other_responses(context);

                // Create the this channel
                let (our_channel, our_future) = ExpectedEntityChannel::new(entity_id, Arc::clone(&responses));

                let future = async move {
                    let all_responses = future::join_all(vec![other_future, our_future.boxed()]).await;
                    all_responses.into_iter()
                        .fold(Ok(()), fold_recipe_error)
                };

                ((other_channel1, other_channel2, other_channel3, our_channel.boxed()), future.boxed())
            }),
        }
    }
}
