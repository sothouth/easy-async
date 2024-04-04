#[test]
fn test() {
    use std::mem::size_of;
    trait SomeTrait {}
    println!("&i32:{}", size_of::<&i32>());
    println!("&[i32]:{}", size_of::<&[i32]>());
    println!("Box<i32>:{}", size_of::<Box<i32>>());
    println!("&Box<i32>:{}", size_of::<&Box<i32>>());
    println!("&dyn Trait:{}", size_of::<&dyn SomeTrait>());
    println!("&[&dyn Trait]:{}", size_of::<&[&dyn SomeTrait]>());
    println!("Box<Trait>:{}", size_of::<Box<dyn SomeTrait>>());
    println!("[&dyn Trait;4]:{}", size_of::<[&dyn SomeTrait; 4]>());
    println!("[i32;4]:{}", size_of::<[i32; 4]>());
}

#[test]
fn test_vtable() {
    trait Test {
        fn add(&self) -> i32;
        fn sub(&self) -> i32;
        fn mul(&self) -> i32;
        fn div(&self) -> i32;
    }

    #[repr(C)]
    struct FatPointer<'a> {
        data: &'a mut Data,
        vtable: *const usize,
    }
    struct Data {
        a: i32,
        b: i32,
    }
    fn add(s: &Data) -> i32 {
        s.a + s.b
    }

    fn sub(s: &Data) -> i32 {
        s.a - s.b
    }

    fn mul(s: &Data) -> i32 {
        s.a * s.b
    }

    fn div(s: &Data) -> i32 {
        s.a / s.b
    }

    let mut data = Data { a: 3, b: 2 };
    let vtable = vec![
        0,
        7,
        8,
        add as usize,
        sub as usize,
        mul as usize,
        div as usize,
    ];

    let fat = FatPointer {
        data: &mut data,
        vtable: vtable.as_ptr(),
    };

    let p = unsafe { std::mem::transmute::<FatPointer, &dyn Test>(fat) };

    assert_eq!(p.add(), 5);
    assert_eq!(p.sub(), 1);
    assert_eq!(p.mul(), 6);
    assert_eq!(p.div(), 1);
}

fn main() {}
