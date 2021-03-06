// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use dbus::arg::IterAppend;
use dbus_tree::{MTSync, MethodErr, PropInfo};

use crate::dbus_api::{
    pool::shared::{self, get_pool_property},
    types::TData,
};

pub fn get_pool_name(
    i: &mut IterAppend,
    p: &PropInfo<MTSync<TData>, TData>,
) -> Result<(), MethodErr> {
    get_pool_property(i, p, |(name, _, _)| Ok(shared::pool_name_prop(&name)))
}
