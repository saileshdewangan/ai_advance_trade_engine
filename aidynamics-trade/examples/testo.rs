use rand::Rng;

#[derive(Debug)]
struct Person {
    name: String,
    age: u32,
}

fn main() {
    let mut people = vec![Person {
        name: String::from("Alice"),
        age: 30,
    }];

    let mut i = 0;
    while i <= 1000 {
        let mut rng = rand::thread_rng();
        // Define the character set
        let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            .chars()
            .collect();

        // Generate a random length between 1 and 10
        let len = rng.gen_range(1..=10);

        // Generate a random string
        let random_string: String = (0..len)
            .map(|_| chars[rng.gen_range(0..chars.len())])
            .collect();

        people.push(Person {
            name: String::from(random_string),
            age: i,
        });
        i = i + 1;
    }
    people.push(Person {
        name: String::from("Bob"),
        age: 24,
    });
    people.push(Person {
        name: String::from("Charlie"),
        age: 29,
    });
    people.push(Person {
        name: String::from("David"),
        age: 30,
    });

    // Filter the people vector to include only those who are 30 years old
    // let filtered_people: Vec<&Person> = people.iter().filter(|&person| person.age > 60).collect();

    loop {
        let mut rng = rand::thread_rng();

        // Generate a random number between 1 and 1000
        let random_number = rng.gen_range(1..=1000);

        let filtered_people: Vec<&Person> = people
            .iter()
            .filter(|&person| person.age >= random_number)
            .collect();
        for v in filtered_people {
            println!("Randome No : {:?}, Name : {:?}", v.age, v.name);
        }
    }
    // println!("{:?}", filtered_people);
}
