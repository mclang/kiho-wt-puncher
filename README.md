# Kiho Worktime Puncher

Simple Rust command line application that can be used to make Kiho
worktime `LOGIN` and `LOGOUT` punch lines using Kiho HTTP API.
Running the application first time creates sample TOML configuration file,
path of which is printed out when using verbose (`-v`) mode flag. Thus best
command to start with is something like `kiho-worktime -v get config`.

Command line argument parsing is done using `clap` crate, which handles error
cases and generates `--help` for each command and sub-command automatically.

**Some examples:**
```
$ kiho-worktime get config
$ kiho-worktime get lastest 10 login
$ kiho-worktime start "Things to do, places to be - meetings to attend :/"
$ kiho-worktime -dv stop
$ kiho-worktime --help
```


## Rust Design Idioms and Patterns

Taken from Rust [Unofficial Patterns](https://rust-unofficial.github.io/patterns/intro.html) book.
Check also [Comprehensive Rust](https://google.github.io/comprehensive-rust/).

### Rust Idioms - TL;DR
1. Use borrowed types for arguments
  - Avoids using additional layer of indirection and makes functions more reusable
  - `$String` -> `&str` (immutable string slice)
  - `&Vec<T>` -> `&[T]`
  - `&Box<t>` -> `&T`
  - Check also [Smart Pointers](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html) (Rust Book)
2. Concatenating strings with `format!`
  - Using `push` might be faster, but many times `format!` is more readable
3. Constructors
  - Rust convention is to use an [associated function](https://doc.rust-lang.org/stable/book/ch05-03-method-syntax.html#associated-functions) `new` to create an objects
  - Rust supports default constructors with the `Default` trait
  - It is common to implement both because users expect `new`
  - The advantage of implementing or deriving `Default` is that your type can now be used with `or_default` -functions
  - Check also [builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html) pattern
4. The `Default` trait
  - Makes type usable with containers and other generic types, e.g `Option::unwrap_or_default()`
  - Can be done automatically with `#[derive(Default)]` for structs whose fields all also implement it
  - Note that constructors, e.g `new` can have multiple arguments but `Default` does not
5. Collections are smart pointers
  - Use the `Deref` trait to treat collections like smart pointers, offering owning and borrowed views of data.
  - Most methods you might expect to be implemented for `Vecs` are instead implemented for slices.
  - Offering a borrowed view of that data allows for more flexible APIs.
  - For example, `String` and `&str` has this relation
6. Finalisation in destructors
  - Rust does not provide the equivalent to `finally` blocks
  - Implement destructor, i.e `Drop` trait whenever necessary.
  - Handles `panic!`, early returns, etc but still **not guaranteed** to run
  - See also [RAII guards](https://rust-unofficial.github.io/patterns/patterns/behavioural/RAII.html)
7. Use `mem::{take(_), replace(_)}` to keep owned values in changed enums
  - Because _clone to satisfy borrow checker_ is an anti-pattern
  - Usable when `enum` has more than one variants (e.g `A { name: String, x: u8 }` and `B { name: String }`)
  - Avoids extra allocation
8. On-Stack Dynamic Dispatch
  - Rust can dynamically dispatch over multiple values
  - Check the [example](https://rust-unofficial.github.io/patterns/idioms/on-stack-dyn-dispatch.html#example)
9. Foreign Function Interface (FFI)
  - [Idiomatic Errors](https://rust-unofficial.github.io/patterns/idioms/ffi/errors.html)
  - [Accepting String](https://rust-unofficial.github.io/patterns/idioms/ffi/accepting-strings.html) with minimal unsafe code
  - [Passing Strings](https://rust-unofficial.github.io/patterns/idioms/ffi/passing-strings.html) to FFI functions
  - TL;DR: Borrow instead of giving ownership and minimize `unsafe` code blocks
10. Iterating over an Option
  - Since Option implements `IntoIterator`, it can be used as an argument to `.extend()`
  - If you need to tack an `Option` to the end of an existing iterator, you can pass it to `.chain()`
  - Also, since `Option` implements `IntoIterator`, it’s possible to iterate over it using a for `loop`.
11. Pass Variables to closures
  - By default, closures capture their environment by borrowing, but you can use `move` -closure to move whole environment
  - Prefer variable rebinding in separate scope to give the closure a copy of the data or pass data by reference selectively
12. Privacy for extensibility
  - May be needed if you want to add public fields into a public `struct` or new variants into `enum` **without** breaking backwards compatibility
  - Use `#[non_exhaustive]` on `struct`s, `enum`s, and `enum` variants.
  - Note that `#[non_exhaustive]` works only across crate boundaries. Within a crate, use private field method instead.
  - Use this deliberately and with caution: incrementing the major version when adding fields or variants is often a better option.
13. Easy doc initialization
  - If a struct takes significant effort to initialize when writing docs, it can be quicker to wrap your example with a helper function which takes the struct as an argument.
  - Check the [example](https://rust-unofficial.github.io/patterns/idioms/rustdoc-init.html#example) to understand
14. Temporary mutability
  - Sometimes data needs to be modified during initialisation but still be immutable afterwards.
  - Use nested block or variable rebinding.
15. Return consumed args on error
  - If a fallible function consumes (moves) an argument, return that argument back inside an error.
  - This makes it possible to re-try some alternative method without the need to clone data for every call.
  - The standard library uses this approach in e.g. `String::from_utf8` method.


### Design Patterns - TL;DR

> Design patterns are “general reusable solutions to a commonly occurring problem within a given context in software design”.

> If overused, design patterns can add unnecessary complexity to programs. However, they are
> a great way to share intermediate and advanced level knowledge about a programming language.

> YAGNI is an acronym that stands for "You Aren't Going to Need It". It’s a vital software design principle to apply as you write code.

**Behavioural**
1. [Command](https://rust-unofficial.github.io/patterns/patterns/behavioural/command.html)
  - The basic idea of the Command pattern is to separate out actions into its own objects and pass them as parameters.
  - Can be done e.g using Trait objects or functions pointers.
2. [Interpreter](https://rust-unofficial.github.io/patterns/patterns/behavioural/interpreter.html)
  - If a problem occurs very often and requires long and repetitive steps to solve it, [DSL](https://en.wikipedia.org/wiki/Domain-specific_language) and an interpreter might be the way to go
3. [Newtype](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html)
  - The primary motivation for newtypes is abstraction.
  - Different thing than plain `type` alias.
  - For example by implementing `Display` for `struct Password(String)` you can hide password strings
  - Newtypes can be used for distinguishing units, e.g., wrapping `f64` to give distinguishable Miles and Kilometres
  - Newtypes are a zero-cost abstraction - there is no runtime overhead.
4. [RAII Guards](https://rust-unofficial.github.io/patterns/patterns/behavioural/RAII.html)
  - Stands for "Resource Acquisition is Initialisation"
  - Resource initialization is done in constructor (i.e `new`) and finalization in destructor (i.e `drop`)
5. [Strategy](https://rust-unofficial.github.io/patterns/patterns/behavioural/strategy.html)
  - Also known as _Policy_ pattern.
  - Technique that enables _Separation of Concerns_.
  - Allows also to decouple software modules through _Dependency Inversion_.
  - Usually done using Traits, from which `server` is a good example.
6. [Visitor](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html)
  - A visitor encapsulates an algorithm that operates over a heterogeneous collection of objects.
  - It allows multiple different algorithms to be written over the same data.
  - Allows separating the traversal of a collection of objects from the operations performed on each object.
  - The visitor pattern is useful anywhere that you want to apply an algorithm to heterogeneous data.
  - The fold pattern below is similar to visitor but produces a new version of the visited data structure.

**Creational**
1. [Builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html)
  - Construct an object with calls to a builder helper.
  - For example, construct `Foo` using `FooBuilder` by setting things and calling `build` in the end.
  - Useful when you would otherwise require many constructors or where construction has side effects.
2. [Fold](https://rust-unofficial.github.io/patterns/patterns/creational/fold.html)
  - Not the same than `fold` method that iterators have but like `map` with extra flexibility.
  - Closely related to Visitor pattern but either creates new collection or modifies existing one.

**Structural**
1. [Compose Structs](https://rust-unofficial.github.io/patterns/patterns/structural/compose-structs.html)
  - Sometimes a large struct will cause issues with the borrow checker.
  - This pattern is most useful, when you have a struct that ended up with a lot of fields that you want to borrow independently.
  - Decomposition of structs lets you work around limitations in the borrow checker. And it often produces a better design.
2. [Prefer Small Crates](https://rust-unofficial.github.io/patterns/patterns/structural/small-crates.html)
  - Small crates are easier to understand, and encourage more modular code.
  - The compilation unit of Rust is the crate, thus multiple crates allow parallel builds.
3. [Contain unsafety in small modules](https://rust-unofficial.github.io/patterns/patterns/structural/unsafe-mods.html)
  - If you have `unsafe` code, create the smallest possible module that can uphold the needed invariants.
  - This restricts the unsafe code that must be audited.
  - Writing the outer module is much easier, since you can count on the guarantees of the inner module.

**Foreign Function Interface (FFI)**
1. [Object-Based APIs](https://rust-unofficial.github.io/patterns/patterns/ffi/export.html)
  - Rust has built-in FFI support to other languages.
  - Rust APIs which are exposed to other languages, have some important design principles which differ from normal Rust API design.
  - The Object-Based API design allows for writing shims that have good memory safety characteristics, and a clean boundary of what is `safe` and what is `unsafe`.
2. [Type Consolidation into Wrappers](https://rust-unofficial.github.io/patterns/patterns/ffi/wrappers.html)
  - Designed to allow gracefully handling multiple related types, while minimizing the surface area for memory unsafety.
  - Makes APIs safer to use, avoiding issues with lifetimes between types.


### Anti-Patterns - TL;DR

> An anti-pattern is a solution to a “recurring problem that is usually ineffective and risks being highly counterproductive”.

1. [Clone to satisfy the borrow checker](https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html)
  - This anti-pattern arises when the developer resolves the borrow checker error by cloning the variable.
  - Using `.clone()` causes a copy of the data to be made.
  - Note though that `Rc<T>` and `Arc<T>` handle `clone` intelligently!
  - Using `cargo clippy` might help you to solve the issue better way.
2. [#[deny(warnings)]](https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html)
  - A well-intentioned crate author wants to ensure their code builds without warnings...
  - It is short and will stop the build if anything is amiss.
  - Using `cargo clippy` might help also with this.
3. [Deref Polymorphism](https://rust-unofficial.github.io/patterns/anti_patterns/deref.html)
  - Misuse the `Deref` trait to emulate inheritance between structs, and thus reuse methods.
  - Surprising idiom that future programmers will not except b/c it's against how `Deref` trait is intended to be used.
  - Note: There is no one good alternative (yet) :/


### Functional Programming

> Rust is an imperative language, but it follows many functional programming paradigms.

1. [Programming paradigms](https://rust-unofficial.github.io/patterns/functional/paradigms.html)
  - Imperative programs describe **how** to do something, whereas declarative programs describe **what** to do.
2. [Generics as Type Classes](https://rust-unofficial.github.io/patterns/functional/generics-type-classes.html)
  - Rust’s type system is designed more like functional languages (like Haskell) rather than imperative languages (like Java and C++).
3. [Functional Optics](https://rust-unofficial.github.io/patterns/functional/optics.html)
  - Optics is a type of API design that is common to functional languages.
  - This is a pure functional concept that is not frequently used in Rust.
  - Quite large topic, needs understanding language design...
  - Check `Serde`-API for example.


