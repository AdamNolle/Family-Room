import SwiftUI

struct EventDetailView: View {
    let event: FamilyEvent
    @Environment(\.dismiss) var dismiss
    
    var body: some View {
        ZStack {
            AdaptiveFabricBackground(type: .linen)
            
            ScrollView {
                VStack(alignment: .leading, spacing: 30) {
                    // Back Button
                    Button {
                        dismiss()
                    } label: {
                        HStack {
                            Image(systemName: "arrow.left")
                            Text("Back to Home")
                        }
                        .font(.subheadline.bold())
                        .foregroundStyle(FurnitureDesign.walnutWood)
                    }
                    .padding(.horizontal)
                    
                    // Title
                    VStack(alignment: .leading, spacing: 8) {
                        Text(event.name)
                            .font(.system(size: 40, weight: .black, design: .serif))
                            .foregroundStyle(FurnitureDesign.walnutWood)
                        Text("\(event.date) • Family Album")
                            .font(.headline)
                            .foregroundStyle(FurnitureDesign.sageWool)
                    }
                    .padding(.horizontal)
                    
                    // Physical Photos Grid
                    LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 20) {
                        ForEach(0..<12) { i in
                            VStack {
                                Rectangle()
                                    .fill(FurnitureDesign.softLinen)
                                    .aspectRatio(1, contentMode: .fit)
                                    .overlay(
                                        Image(systemName: "photo")
                                            .foregroundStyle(FurnitureDesign.walnutWood.opacity(0.2))
                                    )
                                    .padding(8)
                                    .background(.white)
                                    .clipShape(Rectangle())
                                    .shadow(radius: 3)
                                    .rotationEffect(.degrees(Double.random(in: -2...2)))
                            }
                        }
                    }
                    .padding(.horizontal)
                }
                .padding(.top, 40)
            }
        }
    }
}
