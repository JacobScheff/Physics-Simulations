let grids_to_check = [5, 3]

// for(let i = -grids_to_check[0]; i <= grids_to_check[0]; i++) {
//     for(let j = -grids_to_check[1]; j <= grids_to_check[1]; j++) {
//         console.log(i, j)
//     }
// }

// 1d loop
for(let i = 0; i < (grids_to_check[0] * 2 + 1) * (grids_to_check[1] * 2 + 1); i++) {
    let x = Math.floor(i / (grids_to_check[1] * 2 + 1)) - grids_to_check[0]
    let y = i % (grids_to_check[1] * 2 + 1) - grids_to_check[1]
    console.log(x, y)
}