use gamacros_bit_mask::{Bitmask, Bitable};

/// Simple example demonstrating basic Bitmask usage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Permission {
    Read,
    Write,
    Execute,
    Delete,
}

impl Bitable for Permission {
    fn bit(&self) -> u64 {
        match self {
            Permission::Read => 1 << 0,
            Permission::Write => 1 << 1,
            Permission::Execute => 1 << 2,
            Permission::Delete => 1 << 3,
        }
    }

    fn index(&self) -> u32 {
        match self {
            Permission::Read => 0,
            Permission::Write => 1,
            Permission::Execute => 2,
            Permission::Delete => 3,
        }
    }
}

fn main() {
    // Create permissions for a file
    let file_perms = Bitmask::new(&[Permission::Read, Permission::Write]);
    println!("File permissions: {file_perms:?}");

    // Check permissions
    println!("Can read: {}", file_perms.contains(Permission::Read));
    println!("Can write: {}", file_perms.contains(Permission::Write));
    println!("Can execute: {}", file_perms.contains(Permission::Execute));

    // Add execute permission
    let mut updated_perms = file_perms;
    updated_perms.insert(Permission::Execute);
    println!("Updated permissions: {updated_perms:?}");

    // Check if file permissions are subset of admin permissions
    let admin_perms = Bitmask::new(&[
        Permission::Read,
        Permission::Write,
        Permission::Execute,
        Permission::Delete,
    ]);
    println!(
        "File perms is subset of admin: {}",
        updated_perms.is_subset(&admin_perms)
    );

    // Remove write permission
    updated_perms.remove(Permission::Write);
    println!("After removing write: {updated_perms:?}");
}
