use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;

use anyhow::{Context, Result};

pub trait Resource {
    fn fmt_row(&self, formatter: &mut Formatter<'_>) -> fmt::Result;
    fn fmt_detail(&self, formatter: &mut Formatter<'_>) -> fmt::Result;

    fn row(&self) -> Row<'_, Self>
    where
        Self: Sized,
    {
        Row(self)
    }

    fn detail(&self) -> Detail<'_, Self>
    where
        Self: Sized,
    {
        Detail(self)
    }
}

pub struct Row<'a, R: Resource>(&'a R);
pub struct Detail<'a, R: Resource>(&'a R);

impl<R: Resource> Display for Row<'_, R> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt_row(formatter)
    }
}

impl<R: Resource> Display for Detail<'_, R> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt_detail(formatter)
    }
}

pub trait ClientFactory: Sized {
    fn connect(profile: Option<&str>) -> Result<Self>;
}

pub trait Locator<R, C>
where
    R: Resource,
{
    fn resolve(self, client: &C) -> Result<R>;
    fn web_url(&self, client: &C) -> Result<String>;
}

pub struct Loaded<L, R> {
    locator: L,
    resource: R,
}

impl<L, R> Loaded<L, R> {
    pub fn new(locator: L, resource: R) -> Self {
        Self { locator, resource }
    }
}

impl<L, R, C> Locator<R, C> for Loaded<L, R>
where
    R: Resource,
    L: Locator<R, C>,
{
    fn resolve(self, _client: &C) -> Result<R> {
        Ok(self.resource)
    }

    fn web_url(&self, client: &C) -> Result<String> {
        self.locator.web_url(client)
    }
}

pub trait ResourceSpec<R>
where
    R: Resource,
{
    type Client: ClientFactory;
    type ListArgs;
    type ViewArgs;
    type ListedLocator: Locator<R, Self::Client>;
    type ViewLocator: Locator<R, Self::Client>;
    type ListIter: IntoIterator<Item = Self::ListedLocator>;

    fn list(client: &Self::Client, args: Self::ListArgs) -> Result<Self::ListIter>;
    fn locate(args: Self::ViewArgs) -> Self::ViewLocator;
}

#[derive(Clone, Copy)]
pub enum ViewTarget {
    Terminal,
    Web,
}

impl From<bool> for ViewTarget {
    fn from(web: bool) -> Self {
        if web { Self::Web } else { Self::Terminal }
    }
}

pub struct ResourceManager<S, R>
where
    R: Resource,
    S: ResourceSpec<R>,
{
    client: S::Client,
    marker: PhantomData<(S, R)>,
}

impl<S, R> ResourceManager<S, R>
where
    R: Resource,
    S: ResourceSpec<R>,
{
    pub fn connect(profile: Option<&str>) -> Result<Self> {
        Ok(Self {
            client: S::Client::connect(profile)?,
            marker: PhantomData,
        })
    }

    pub fn list(&self, args: S::ListArgs) -> Result<()> {
        for locator in S::list(&self.client, args)? {
            let resource = locator.resolve(&self.client)?;
            println!("{}", resource.row());
        }
        Ok(())
    }

    pub fn view(&self, args: S::ViewArgs, target: ViewTarget) -> Result<()> {
        let locator = S::locate(args);

        match target {
            ViewTarget::Terminal => {
                let resource = locator.resolve(&self.client)?;
                println!("{}", resource.detail());
                Ok(())
            }
            ViewTarget::Web => {
                let url = locator.web_url(&self.client)?;
                println!("Opening {url} in your browser...");
                open::that(&url).context("failed to open browser")
            }
        }
    }
}
