use ids_generator::IdsGenerator;

fn main() {
	let ids_generator = IdsGenerator::new(1000..20000, vec![]);
	// let x = utils(10, 20);
	println!("{}", ids_generator.next());
}
