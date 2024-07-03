use ids_generator::IdsGenerator;

fn main() {
	let ids_generator = IdsGenerator::new(0..200, (0..3).collect());

	println!("{:?}", ids_generator.container_type());
	// let x = utils(10, 20);
	println!("{}", ids_generator.next());
}
