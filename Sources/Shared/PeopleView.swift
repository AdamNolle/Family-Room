import SwiftUI

struct PeopleView: View {
    @ObservedObject var viewModel: AppViewModel
    
    var body: some View {
        ZStack {
            AdaptiveFabricBackground(type: viewModel.selectedFabric)
            
            VStack(alignment: .leading, spacing: 30) {
                Text("Family Circle")
                    .font(.system(size: 34, weight: .black, design: .serif))
                    .foregroundStyle(FurnitureDesign.walnutWood)
                    .padding(.horizontal)
                    .padding(.top, 40)
                
                Text("Identified family members")
                    .font(.subheadline)
                    .foregroundStyle(FurnitureDesign.sageWool)
                    .padding(.horizontal)
                
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 30) {
                        ForEach(viewModel.people) { person in
                            VStack(spacing: 15) {
                                Circle()
                                    .fill(FurnitureDesign.sageWool.opacity(0.1))
                                    .frame(width: 120, height: 120)
                                    .overlay(
                                        Text(String(person.name.prefix(1)))
                                            .font(.system(size: 40, weight: .bold, design: .serif))
                                            .foregroundStyle(FurnitureDesign.walnutWood)
                                    )
                                    .overlay(
                                        Circle()
                                            .stroke(FurnitureDesign.walnutWood.opacity(0.3), lineWidth: 1)
                                    )
                                    .shadow(color: .black.opacity(0.1), radius: 10, x: 0, y: 5)
                                
                                Text(person.name)
                                    .font(.headline.serif())
                                    .foregroundStyle(FurnitureDesign.walnutWood)
                            }
                        }
                    }
                    .padding()
                }
                
                Spacer()
            }
        }
    }
}
