use crate::{
    data::npc::{Controller, Npc, NpcId},
    RtState,
};
use common::resources::{Time, TimeOfDay};
use std::{any::Any, marker::PhantomData, ops::ControlFlow};
use world::{IndexRef, World};

/// The context provided to an [`Action`] while it is being performed. It should
/// be possible to access any and all important information about the game world
/// through this struct.
pub struct NpcCtx<'a> {
    pub state: &'a RtState,
    pub world: &'a World,
    pub index: IndexRef<'a>,

    pub time_of_day: TimeOfDay,
    pub time: Time,

    pub npc_id: NpcId,
    pub npc: &'a Npc,
    pub controller: &'a mut Controller,
}

/// A trait that describes 'actions': long-running tasks performed by rtsim
/// NPCs. These can be as simple as walking in a straight line between two
/// locations or as complex as taking part in an adventure with players or
/// performing an entire daily work schedule.
///
/// Actions are built up from smaller sub-actions via the combinator methods
/// defined on this trait, and with the standalone functions in this module.
/// Using these combinators, in a similar manner to using the [`Iterator`] API,
/// it is possible to construct arbitrarily complex actions including behaviour
/// trees (see [`choose`] and [`watch`]) and other forms of moment-by-moment
/// decision-making.
///
/// On completion, actions may produce a value, denoted by the type parameter
/// `R`. For example, an action may communicate whether it was successful or
/// unsuccessful through this completion value.
///
/// You should not need to implement this trait yourself when writing AI code.
/// If you find yourself wanting to implement it, please discuss with the core
/// dev team first.
pub trait Action<R = ()>: Any + Send + Sync {
    /// Returns `true` if the action should be considered the 'same' (i.e:
    /// achieving the same objective) as another. In general, the AI system
    /// will try to avoid switching (and therefore restarting) tasks when the
    /// new task is the 'same' as the old one.
    // TODO: Figure out a way to compare actions based on their 'intention': i.e:
    // two pathing actions should be considered equivalent if their destination
    // is the same regardless of the progress they've each made.
    fn is_same(&self, other: &Self) -> bool
    where
        Self: Sized;

    /// Like [`Action::is_same`], but allows for dynamic dispatch.
    fn dyn_is_same_sized(&self, other: &dyn Action<R>) -> bool
    where
        Self: Sized,
    {
        match (other as &dyn Any).downcast_ref::<Self>() {
            Some(other) => self.is_same(other),
            None => false,
        }
    }

    /// Like [`Action::is_same`], but allows for dynamic dispatch.
    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool;

    /// Reset the action to its initial state such that it can be repeated.
    fn reset(&mut self);

    /// Perform the action for the current tick.
    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R>;

    /// Create an action that chains together two sub-actions, one after the
    /// other.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Walk toward an enemy NPC and, once done, attack the enemy NPC
    /// goto(enemy_npc).then(attack(enemy_npc))
    /// ```
    fn then<A1: Action<R1>, R1>(self, other: A1) -> Then<Self, A1, R>
    where
        Self: Sized,
    {
        Then {
            a0: self,
            a0_finished: false,
            a1: other,
            phantom: PhantomData,
        }
    }

    /// Create an action that repeats a sub-action indefinitely.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Endlessly collect flax from the environment
    /// find_and_collect(ChunkResource::Flax).repeat()
    /// ```
    fn repeat<R1>(self) -> Repeat<Self, R1>
    where
        Self: Sized,
    {
        Repeat(self, PhantomData)
    }

    /// Stop the sub-action suddenly if a condition is reached.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Keep going on adventures until your 111th birthday
    /// go_on_an_adventure().repeat().stop_if(|ctx| ctx.npc.age > 111.0)
    /// ```
    fn stop_if<F: FnMut(&mut NpcCtx) -> bool>(self, f: F) -> StopIf<Self, F>
    where
        Self: Sized,
    {
        StopIf(self, f)
    }

    /// Map the completion value of this action to something else.
    fn map<F: FnMut(R) -> R1, R1>(self, f: F) -> Map<Self, F, R>
    where
        Self: Sized,
    {
        Map(self, f, PhantomData)
    }

    /// Box the action. Often used to perform type erasure, such as when you
    /// want to return one of many actions (each with different types) from
    /// the same function.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Error! Type mismatch between branches
    /// if npc.is_too_tired() {
    ///     goto(npc.home)
    /// } else {
    ///     go_on_an_adventure()
    /// }
    ///
    /// // All fine
    /// if npc.is_too_tired() {
    ///     goto(npc.home).boxed()
    /// } else {
    ///     go_on_an_adventure().boxed()
    /// }
    /// ```
    fn boxed(self) -> Box<dyn Action<R>>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

impl<R: 'static> Action<R> for Box<dyn Action<R>> {
    fn is_same(&self, other: &Self) -> bool { (**self).dyn_is_same(other) }

    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool {
        match (other as &dyn Any).downcast_ref::<Self>() {
            Some(other) => self.is_same(other),
            None => false,
        }
    }

    fn reset(&mut self) { (**self).reset(); }

    // TODO: Reset closure state?
    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R> { (**self).tick(ctx) }
}

// Now

/// See [`now`].
#[derive(Copy, Clone)]
pub struct Now<F, A>(F, Option<A>);

impl<R: Send + Sync + 'static, F: FnMut(&mut NpcCtx) -> A + Send + Sync + 'static, A: Action<R>>
    Action<R> for Now<F, A>
{
    // TODO: This doesn't compare?!
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.1 = None; }

    // TODO: Reset closure state?
    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R> {
        (self.1.get_or_insert_with(|| (self.0)(ctx))).tick(ctx)
    }
}

/// Start a new action based on the state of the world (`ctx`) at the moment the
/// action is started.
///
/// If you're in a situation where you suddenly find yourself needing `ctx`, you
/// probably want to use this.
///
/// # Example
///
/// ```ignore
/// // An action that makes an NPC immediately travel to its *current* home
/// now(|ctx| goto(ctx.npc.home))
/// ```
pub fn now<F, A>(f: F) -> Now<F, A>
where
    F: FnMut(&mut NpcCtx) -> A,
{
    Now(f, None)
}

// Just

/// See [`just`].
#[derive(Copy, Clone)]
pub struct Just<F, R = ()>(F, PhantomData<R>);

impl<R: Send + Sync + 'static, F: FnMut(&mut NpcCtx) -> R + Send + Sync + 'static> Action<R>
    for Just<F, R>
{
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) {}

    // TODO: Reset closure state?
    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R> { ControlFlow::Break((self.0)(ctx)) }
}

/// An action that executes some code just once when performed.
///
/// If you want to execute this code on every tick, consider combining it with
/// [`Action::repeat`].
///
/// # Example
///
/// ```ignore
/// // Make the current NPC say 'Hello, world!' exactly once
/// just(|ctx| ctx.controller.say("Hello, world!"))
/// ```
pub fn just<F, R: Send + Sync + 'static>(mut f: F) -> Just<F, R>
where
    F: FnMut(&mut NpcCtx) -> R + Send + Sync + 'static,
{
    Just(f, PhantomData)
}

// Finish

/// See [`finish`].
#[derive(Copy, Clone)]
pub struct Finish;

impl Action<()> for Finish {
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<()>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) {}

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<()> { ControlFlow::Break(()) }
}

/// An action that immediately finishes without doing anything.
///
/// This action is useless by itself, but becomes useful when combined with
/// actions that make decisions.
///
/// # Example
///
/// ```ignore
/// now(|ctx| {
///     if ctx.npc.is_tired() {
///         sleep().boxed() // If we're tired, sleep
///     } else if ctx.npc.is_hungry() {
///         eat().boxed() // If we're hungry, eat
///     } else {
///         finish().boxed() // Otherwise, do nothing
///     }
/// })
/// ```
pub fn finish() -> Finish { Finish }

// Tree

pub type Priority = usize;

pub const URGENT: Priority = 0;
pub const IMPORTANT: Priority = 1;
pub const CASUAL: Priority = 2;

pub struct Node<R>(Box<dyn Action<R>>, Priority);

/// Perform an action with [`URGENT`] priority (see [`choose`]).
pub fn urgent<A: Action<R>, R>(a: A) -> Node<R> { Node(Box::new(a), URGENT) }

/// Perform an action with [`IMPORTANT`] priority (see [`choose`]).
pub fn important<A: Action<R>, R>(a: A) -> Node<R> { Node(Box::new(a), IMPORTANT) }

/// Perform an action with [`CASUAL`] priority (see [`choose`]).
pub fn casual<A: Action<R>, R>(a: A) -> Node<R> { Node(Box::new(a), CASUAL) }

/// See [`choose`] and [`watch`].
pub struct Tree<F, R> {
    next: F,
    prev: Option<Node<R>>,
    interrupt: bool,
}

impl<F: FnMut(&mut NpcCtx) -> Node<R> + Send + Sync + 'static, R: 'static> Action<R>
    for Tree<F, R>
{
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.prev = None; }

    // TODO: Reset `next` too?
    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R> {
        let new = (self.next)(ctx);

        let prev = match &mut self.prev {
            Some(prev) if prev.1 <= new.1 && (prev.0.dyn_is_same(&*new.0) || !self.interrupt) => {
                prev
            },
            _ => self.prev.insert(new),
        };

        match prev.0.tick(ctx) {
            ControlFlow::Continue(()) => return ControlFlow::Continue(()),
            ControlFlow::Break(r) => {
                self.prev = None;
                ControlFlow::Break(r)
            },
        }
    }
}

/// An action that allows implementing a decision tree, with action
/// prioritisation.
///
/// The inner function will be run every tick to decide on an action. When an
/// action is chosen, it will be performed until completed *UNLESS* an action
/// with a more urgent priority is chosen in a subsequent tick. [`choose`] tries
/// to commit to actions when it can: only more urgent actions will interrupt an
/// action that's currently being performed. If you want something that's more
/// eager to switch actions, see [`watch`].
///
/// # Example
///
/// ```ignore
/// choose(|ctx| {
///     if ctx.npc.is_being_attacked() {
///         urgent(combat()) // If we're in danger, do something!
///     } else if ctx.npc.is_hungry() {
///         important(eat()) // If we're hungry, eat
///     } else {
///         casual(idle()) // Otherwise, do nothing
///     }
/// })
/// ```
pub fn choose<R: 'static, F>(f: F) -> impl Action<R>
where
    F: FnMut(&mut NpcCtx) -> Node<R> + Send + Sync + 'static,
{
    Tree {
        next: f,
        prev: None,
        interrupt: false,
    }
}

/// An action that allows implementing a decision tree, with action
/// prioritisation.
///
/// The inner function will be run every tick to decide on an action. When an
/// action is chosen, it will be performed until completed unless a different
/// action is chosen in a subsequent tick. [`watch`] is very unfocussed and will
/// happily switch between actions rapidly between ticks if conditions change.
/// If you want something that tends to commit to actions until they are
/// completed, see [`choose`].
///
/// # Example
///
/// ```ignore
/// choose(|ctx| {
///     if ctx.npc.is_being_attacked() {
///         urgent(combat()) // If we're in danger, do something!
///     } else if ctx.npc.is_hungry() {
///         important(eat()) // If we're hungry, eat
///     } else {
///         casual(idle()) // Otherwise, do nothing
///     }
/// })
/// ```
pub fn watch<R: 'static, F>(f: F) -> impl Action<R>
where
    F: FnMut(&mut NpcCtx) -> Node<R> + Send + Sync + 'static,
{
    Tree {
        next: f,
        prev: None,
        interrupt: true,
    }
}

// Then

/// See [`Action::then`].
#[derive(Copy, Clone)]
pub struct Then<A0, A1, R0> {
    a0: A0,
    a0_finished: bool,
    a1: A1,
    phantom: PhantomData<R0>,
}

impl<A0: Action<R0>, A1: Action<R1>, R0: Send + Sync + 'static, R1: Send + Sync + 'static>
    Action<R1> for Then<A0, A1, R0>
{
    fn is_same(&self, other: &Self) -> bool {
        self.a0.is_same(&other.a0) && self.a1.is_same(&other.a1)
    }

    fn dyn_is_same(&self, other: &dyn Action<R1>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) {
        self.a0.reset();
        self.a0_finished = false;
        self.a1.reset();
    }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R1> {
        if !self.a0_finished {
            match self.a0.tick(ctx) {
                ControlFlow::Continue(()) => return ControlFlow::Continue(()),
                ControlFlow::Break(_) => self.a0_finished = true,
            }
        }
        self.a1.tick(ctx)
    }
}

// Repeat

/// See [`Action::repeat`].
#[derive(Copy, Clone)]
pub struct Repeat<A, R = ()>(A, PhantomData<R>);

impl<R: Send + Sync + 'static, A: Action<R>> Action<!> for Repeat<A, R> {
    fn is_same(&self, other: &Self) -> bool { self.0.is_same(&other.0) }

    fn dyn_is_same(&self, other: &dyn Action<!>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.0.reset(); }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<!> {
        match self.0.tick(ctx) {
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
            ControlFlow::Break(_) => {
                self.0.reset();
                ControlFlow::Continue(())
            },
        }
    }
}

// Sequence

/// See [`seq`].
#[derive(Copy, Clone)]
pub struct Sequence<I, A, R = ()>(I, I, Option<A>, PhantomData<R>);

impl<R: Send + Sync + 'static, I: Iterator<Item = A> + Clone + Send + Sync + 'static, A: Action<R>>
    Action<()> for Sequence<I, A, R>
{
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<()>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) {
        self.0 = self.1.clone();
        self.2 = None;
    }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<()> {
        let item = if let Some(prev) = &mut self.2 {
            prev
        } else {
            match self.0.next() {
                Some(next) => self.2.insert(next),
                None => return ControlFlow::Break(()),
            }
        };

        if let ControlFlow::Break(_) = item.tick(ctx) {
            self.2 = None;
        }

        ControlFlow::Continue(())
    }
}

/// An action that consumes and performs an iterator of actions in sequence, one
/// after another.
///
/// # Example
///
/// ```ignore
/// // A list of enemies we should attack in turn
/// let enemies = vec![
///     ugly_goblin,
///     stinky_troll,
///     rude_dwarf,
/// ];
///
/// // Attack each enemy, one after another
/// seq(enemies
///     .into_iter()
///     .map(|enemy| attack(enemy)))
/// ```
pub fn seq<I, A, R>(iter: I) -> Sequence<I, A, R>
where
    I: Iterator<Item = A> + Clone,
    A: Action<R>,
{
    Sequence(iter.clone(), iter, None, PhantomData)
}

// StopIf

/// See [`Action::stop_if`].
#[derive(Copy, Clone)]
pub struct StopIf<A, F>(A, F);

impl<A: Action<R>, F: FnMut(&mut NpcCtx) -> bool + Send + Sync + 'static, R> Action<Option<R>>
    for StopIf<A, F>
{
    fn is_same(&self, other: &Self) -> bool { self.0.is_same(&other.0) }

    fn dyn_is_same(&self, other: &dyn Action<Option<R>>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.0.reset(); }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<Option<R>> {
        if (self.1)(ctx) {
            ControlFlow::Break(None)
        } else {
            self.0.tick(ctx).map_break(Some)
        }
    }
}

// Map

/// See [`Action::map`].
#[derive(Copy, Clone)]
pub struct Map<A, F, R>(A, F, PhantomData<R>);

impl<A: Action<R>, F: FnMut(R) -> R1 + Send + Sync + 'static, R: Send + Sync + 'static, R1>
    Action<R1> for Map<A, F, R>
{
    fn is_same(&self, other: &Self) -> bool { self.0.is_same(&other.0) }

    fn dyn_is_same(&self, other: &dyn Action<R1>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.0.reset(); }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R1> {
        self.0.tick(ctx).map_break(&mut self.1)
    }
}
