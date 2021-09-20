mod region;
mod draw;

pub use region::{Region, Cell, Colour, Format};
pub use draw::Draw;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
